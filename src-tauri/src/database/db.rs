use crate::database::schema::init_database;
use crate::error::Error;
use crate::models::{
    AppSettings, Group, LogStream, MetricPoint, Project, ProjectType, Session, SessionWithStats,
};
use rusqlite::{params, Connection, Transaction};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Main database struct managing a single SQLite database
pub struct Database {
    conn: Connection,
    db_path: PathBuf,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(db_path: &Path) -> Result<Self, Error> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = init_database(db_path)?;

        Ok(Self {
            conn,
            db_path: db_path.to_path_buf(),
        })
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Begin a transaction
    pub fn transaction(&mut self) -> Result<Transaction, Error> {
        self.conn
            .unchecked_transaction()
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    // ============================================================================
    // Group Operations
    // ============================================================================

    /// Get all groups with their projects and env vars
    pub fn get_groups(&self) -> Result<Vec<Group>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, directory, sync_file, sync_enabled FROM groups ORDER BY name",
        )?;

        let groups = stmt.query_map([], |row| {
            let group_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let directory: String = row.get(2)?;
            let sync_file: Option<String> = row.get(3)?;
            let sync_enabled: bool = row.get(4)?;

            Ok(Group {
                id: group_id,
                name,
                directory,
                projects: Vec::new(),
                env_vars: HashMap::new(),
                sync_file,
                sync_enabled,
            })
        })?;

        let mut result = Vec::new();
        for group_result in groups {
            let mut group = group_result.map_err(|e| Error::DatabaseError(e.to_string()))?;

            // Load projects for this group
            group.projects = self.get_projects_for_group(&group.id)?;

            // Load env vars for this group
            group.env_vars = self.get_group_env_vars(&group.id)?;

            result.push(group);
        }

        Ok(result)
    }

    /// Get a single group by ID
    pub fn get_group(&self, group_id: &str) -> Result<Option<Group>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, directory, sync_file, sync_enabled FROM groups WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![group_id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let directory: String = row.get(2)?;
            let sync_file: Option<String> = row.get(3)?;
            let sync_enabled: bool = row.get(4)?;

            Ok(Group {
                id,
                name,
                directory,
                projects: Vec::new(),
                env_vars: HashMap::new(),
                sync_file,
                sync_enabled,
            })
        });

        match result {
            Ok(mut group) => {
                group.projects = self.get_projects_for_group(&group.id)?;
                group.env_vars = self.get_group_env_vars(&group.id)?;
                Ok(Some(group))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Create a new group with all its data in a transaction
    pub fn create_group(&mut self, group: &Group) -> Result<(), Error> {
        let tx = self.transaction()?;

        // Insert group
        tx.execute(
            "INSERT INTO groups (id, name, directory, sync_file, sync_enabled) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                group.id,
                group.name,
                group.directory,
                group.sync_file,
                group.sync_enabled as i32
            ],
        )?;

        // Insert env vars
        for (key, value) in &group.env_vars {
            tx.execute(
                "INSERT INTO group_env_vars (group_id, key, value) VALUES (?1, ?2, ?3)",
                params![group.id, key, value],
            )?;
        }

        // Insert projects
        for project in &group.projects {
            Self::insert_project_tx(&tx, &group.id, project)?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Update a group's basic info
    pub fn update_group(
        &self,
        group_id: &str,
        name: &str,
        directory: &str,
        sync_file: Option<&str>,
        sync_enabled: bool,
    ) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE groups SET name = ?1, directory = ?2, sync_file = ?3, sync_enabled = ?4 WHERE id = ?5",
            params![name, directory, sync_file, sync_enabled as i32, group_id],
        )?;
        Ok(())
    }

    /// Rename a group
    pub fn rename_group(&self, group_id: &str, new_name: &str) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE groups SET name = ?1 WHERE id = ?2",
            params![new_name, group_id],
        )?;
        Ok(())
    }

    /// Update group directory
    pub fn update_group_directory(&self, group_id: &str, directory: &str) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE groups SET directory = ?1 WHERE id = ?2",
            params![directory, group_id],
        )?;
        Ok(())
    }

    /// Delete a group and all its associated data (cascade)
    pub fn delete_group(&self, group_id: &str) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
        Ok(())
    }

    /// Update group's sync settings
    pub fn update_group_sync(
        &self,
        group_id: &str,
        sync_file: Option<&str>,
        sync_enabled: bool,
    ) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE groups SET sync_file = ?1, sync_enabled = ?2 WHERE id = ?3",
            params![sync_file, sync_enabled as i32, group_id],
        )?;
        Ok(())
    }

    /// Replace entire group data (used for YAML sync)
    pub fn replace_group(&mut self, group: &Group) -> Result<(), Error> {
        let tx = self.transaction()?;

        // Update group basic info
        tx.execute(
            "UPDATE groups SET name = ?1 WHERE id = ?2",
            params![group.name, group.id],
        )?;

        // Delete and recreate env vars
        tx.execute(
            "DELETE FROM group_env_vars WHERE group_id = ?1",
            params![group.id],
        )?;

        for (key, value) in &group.env_vars {
            tx.execute(
                "INSERT INTO group_env_vars (group_id, key, value) VALUES (?1, ?2, ?3)",
                params![group.id, key, value],
            )?;
        }

        // Get existing project IDs
        let mut stmt = tx.prepare("SELECT id FROM projects WHERE group_id = ?1")?;
        let existing_ids: Vec<String> = stmt
            .query_map(params![group.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);

        // Track which projects still exist
        let mut current_ids: Vec<String> = Vec::new();

        // Update or insert projects
        for project in &group.projects {
            current_ids.push(project.id.clone());

            if existing_ids.contains(&project.id) {
                // Update existing project
                let project_type_str = match project.project_type {
                    ProjectType::Task => "task",
                    ProjectType::Service => "service",
                };

                tx.execute(
                    "UPDATE projects SET name = ?1, command = ?2, auto_restart = ?3, cwd = ?4, project_type = ?5, interactive = ?6 WHERE id = ?7",
                    params![
                        project.name,
                        project.command,
                        project.auto_restart as i32,
                        project.cwd,
                        project_type_str,
                        project.interactive as i32,
                        project.id
                    ],
                )?;

                // Update env vars
                tx.execute(
                    "DELETE FROM project_env_vars WHERE project_id = ?1",
                    params![project.id],
                )?;

                for (key, value) in &project.env_vars {
                    tx.execute(
                        "INSERT INTO project_env_vars (project_id, key, value) VALUES (?1, ?2, ?3)",
                        params![project.id, key, value],
                    )?;
                }
            } else {
                // Insert new project
                Self::insert_project_tx(&tx, &group.id, project)?;
            }
        }

        // Delete projects that no longer exist
        for existing_id in existing_ids {
            if !current_ids.contains(&existing_id) {
                tx.execute("DELETE FROM projects WHERE id = ?1", params![existing_id])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    // ============================================================================
    // Project Operations
    // ============================================================================

    /// Get all projects for a group
    fn get_projects_for_group(&self, group_id: &str) -> Result<Vec<Project>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, auto_restart, cwd, project_type, interactive FROM projects WHERE group_id = ?1 ORDER BY name",
        )?;

        let projects = stmt.query_map(params![group_id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let command: String = row.get(2)?;
            let auto_restart: bool = row.get(3)?;
            let cwd: Option<String> = row.get(4)?;
            let project_type_str: String = row.get(5)?;
            let interactive: bool = row.get(6)?;

            let project_type = match project_type_str.as_str() {
                "task" => ProjectType::Task,
                _ => ProjectType::Service,
            };

            Ok(Project {
                id,
                name,
                command,
                auto_restart,
                cwd,
                project_type,
                interactive,
                env_vars: HashMap::new(),
            })
        })?;

        let mut result = Vec::new();
        for project_result in projects {
            let mut project = project_result.map_err(|e| Error::DatabaseError(e.to_string()))?;
            project.env_vars = self.get_project_env_vars(&project.id)?;
            result.push(project);
        }

        Ok(result)
    }

    /// Get a single project by ID
    pub fn get_project(&self, project_id: &str) -> Result<Option<Project>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, auto_restart, cwd, project_type, interactive FROM projects WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![project_id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let command: String = row.get(2)?;
            let auto_restart: bool = row.get(3)?;
            let cwd: Option<String> = row.get(4)?;
            let project_type_str: String = row.get(5)?;
            let interactive: bool = row.get(6)?;

            let project_type = match project_type_str.as_str() {
                "task" => ProjectType::Task,
                _ => ProjectType::Service,
            };

            Ok(Project {
                id,
                name,
                command,
                auto_restart,
                cwd,
                project_type,
                interactive,
                env_vars: HashMap::new(),
            })
        });

        match result {
            Ok(mut project) => {
                project.env_vars = self.get_project_env_vars(&project.id)?;
                Ok(Some(project))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Get project with its group ID
    pub fn get_project_with_group(
        &self,
        project_id: &str,
    ) -> Result<Option<(Project, String)>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.command, p.auto_restart, p.cwd, p.project_type, p.interactive, p.group_id FROM projects p WHERE p.id = ?1"
        )?;

        let result = stmt.query_row(params![project_id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let command: String = row.get(2)?;
            let auto_restart: bool = row.get(3)?;
            let cwd: Option<String> = row.get(4)?;
            let project_type_str: String = row.get(5)?;
            let interactive: bool = row.get(6)?;
            let group_id: String = row.get(7)?;

            let project_type = match project_type_str.as_str() {
                "task" => ProjectType::Task,
                _ => ProjectType::Service,
            };

            let project = Project {
                id,
                name,
                command,
                auto_restart,
                cwd,
                project_type,
                interactive,
                env_vars: HashMap::new(),
            };

            Ok((project, group_id))
        });

        match result {
            Ok((mut project, group_id)) => {
                project.env_vars = self.get_project_env_vars(&project.id)?;
                Ok(Some((project, group_id)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Create a new project in a transaction
    pub fn create_project(&mut self, group_id: &str, project: &Project) -> Result<(), Error> {
        let tx = self.transaction()?;
        Self::insert_project_tx(&tx, group_id, project)?;
        tx.commit()?;
        Ok(())
    }

    /// Insert a project within a transaction
    fn insert_project_tx(tx: &Transaction, group_id: &str, project: &Project) -> Result<(), Error> {
        let project_type_str = match project.project_type {
            ProjectType::Task => "task",
            ProjectType::Service => "service",
        };

        tx.execute(
            "INSERT INTO projects (id, group_id, name, command, auto_restart, cwd, project_type, interactive) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project.id,
                group_id,
                project.name,
                project.command,
                project.auto_restart as i32,
                project.cwd,
                project_type_str,
                project.interactive as i32
            ],
        )?;

        // Insert env vars
        for (key, value) in &project.env_vars {
            tx.execute(
                "INSERT INTO project_env_vars (project_id, key, value) VALUES (?1, ?2, ?3)",
                params![project.id, key, value],
            )?;
        }

        Ok(())
    }

    /// Update an existing project
    pub fn update_project(&mut self, project: &Project) -> Result<(), Error> {
        let tx = self.transaction()?;

        let project_type_str = match project.project_type {
            ProjectType::Task => "task",
            ProjectType::Service => "service",
        };

        tx.execute(
            "UPDATE projects SET name = ?1, command = ?2, auto_restart = ?3, cwd = ?4, project_type = ?5, interactive = ?6 WHERE id = ?7",
            params![
                project.name,
                project.command,
                project.auto_restart as i32,
                project.cwd,
                project_type_str,
                project.interactive as i32,
                project.id
            ],
        )?;

        // Delete old env vars and insert new ones
        tx.execute(
            "DELETE FROM project_env_vars WHERE project_id = ?1",
            params![project.id],
        )?;

        for (key, value) in &project.env_vars {
            tx.execute(
                "INSERT INTO project_env_vars (project_id, key, value) VALUES (?1, ?2, ?3)",
                params![project.id, key, value],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Delete a project
    pub fn delete_project(&self, project_id: &str) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        Ok(())
    }

    /// Delete multiple projects in a transaction
    pub fn delete_projects(&mut self, project_ids: &[String]) -> Result<(), Error> {
        let tx = self.transaction()?;

        for project_id in project_ids {
            tx.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Convert multiple projects' types in a transaction
    pub fn convert_projects(&mut self, conversions: &[(String, ProjectType)]) -> Result<(), Error> {
        let tx = self.transaction()?;

        for (project_id, project_type) in conversions {
            let project_type_str = match project_type {
                ProjectType::Task => "task",
                ProjectType::Service => "service",
            };
            let auto_restart = *project_type == ProjectType::Service;

            tx.execute(
                "UPDATE projects SET project_type = ?1, auto_restart = ?2 WHERE id = ?3",
                params![project_type_str, auto_restart as i32, project_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    // ============================================================================
    // Environment Variables Operations
    // ============================================================================

    /// Get env vars for a group
    fn get_group_env_vars(&self, group_id: &str) -> Result<HashMap<String, String>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM group_env_vars WHERE group_id = ?1")?;

        let rows = stmt.query_map(params![group_id], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })?;

        let mut result = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(|e| Error::DatabaseError(e.to_string()))?;
            result.insert(key, value);
        }

        Ok(result)
    }

    /// Get env vars for a project
    fn get_project_env_vars(&self, project_id: &str) -> Result<HashMap<String, String>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM project_env_vars WHERE project_id = ?1")?;

        let rows = stmt.query_map(params![project_id], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })?;

        let mut result = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(|e| Error::DatabaseError(e.to_string()))?;
            result.insert(key, value);
        }

        Ok(result)
    }

    /// Update group env vars
    pub fn update_group_env_vars(
        &mut self,
        group_id: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(), Error> {
        let tx = self.transaction()?;

        tx.execute(
            "DELETE FROM group_env_vars WHERE group_id = ?1",
            params![group_id],
        )?;

        for (key, value) in env_vars {
            tx.execute(
                "INSERT INTO group_env_vars (group_id, key, value) VALUES (?1, ?2, ?3)",
                params![group_id, key, value],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Update project env vars
    pub fn update_project_env_vars(
        &mut self,
        project_id: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(), Error> {
        let tx = self.transaction()?;

        tx.execute(
            "DELETE FROM project_env_vars WHERE project_id = ?1",
            params![project_id],
        )?;

        for (key, value) in env_vars {
            tx.execute(
                "INSERT INTO project_env_vars (project_id, key, value) VALUES (?1, ?2, ?3)",
                params![project_id, key, value],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    // ============================================================================
    // Settings Operations
    // ============================================================================

    /// Get all settings
    pub fn get_settings(&self) -> Result<AppSettings, Error> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;

        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })?;

        let mut settings = AppSettings::default();

        for row in rows {
            let (key, value) = row.map_err(|e| Error::DatabaseError(e.to_string()))?;
            match key.as_str() {
                "max_log_lines" => {
                    if let Ok(v) = value.parse() {
                        settings.max_log_lines = v;
                    }
                }
                "editor" => {
                    if !value.is_empty() {
                        settings.editor = Some(value);
                    }
                }
                _ => {}
            }
        }

        Ok(settings)
    }

    /// Update settings
    pub fn update_settings(&self, settings: &AppSettings) -> Result<(), Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('max_log_lines', ?1)",
            params![settings.max_log_lines.to_string()],
        )?;

        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('editor', ?1)",
            params![settings.editor.as_deref().unwrap_or("")],
        )?;

        Ok(())
    }

    // ============================================================================
    // Session Operations
    // ============================================================================

    /// Create a new session
    pub fn create_session(&self, project_id: &str) -> Result<String, Error> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.conn.execute(
            "INSERT INTO sessions (id, project_id, started_at) VALUES (?1, ?2, ?3)",
            params![session_id, project_id, now],
        )?;

        Ok(session_id)
    }

    /// End a session
    pub fn end_session(&self, session_id: &str, exit_status: &str) -> Result<(), Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.conn.execute(
            "UPDATE sessions SET ended_at = ?1, exit_status = ?2 WHERE id = ?3",
            params![now, exit_status, session_id],
        )?;

        Ok(())
    }

    /// Get all sessions for a project
    pub fn get_project_sessions(&self, project_id: &str) -> Result<Vec<Session>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC",
        )?;

        let sessions = stmt.query_map(params![project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        })?;

        let result: Result<Vec<_>, _> = sessions.collect();
        result.map_err(|e| Error::DatabaseError(e.to_string()))
    }

    /// Get sessions with stats for a project
    pub fn get_project_sessions_with_stats(
        &self,
        project_id: &str,
    ) -> Result<Vec<SessionWithStats>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.project_id, s.started_at, s.ended_at, s.exit_status,
                    COALESCE(l.log_count, 0), COALESCE(l.log_size, 0),
                    COALESCE(m.metric_count, 0)
             FROM sessions s
             LEFT JOIN (SELECT session_id, COUNT(*) as log_count, COALESCE(SUM(LENGTH(data)), 0) as log_size FROM logs GROUP BY session_id) l ON l.session_id = s.id
             LEFT JOIN (SELECT session_id, COUNT(*) as metric_count FROM metrics GROUP BY session_id) m ON m.session_id = s.id
             WHERE s.project_id = ?1
             ORDER BY s.started_at DESC"
        )?;

        let sessions = stmt.query_map(params![project_id], |row| {
            Ok(SessionWithStats {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
                log_count: row.get(5)?,
                log_size: row.get(6)?,
                metric_count: row.get(7)?,
            })
        })?;

        let result: Result<Vec<_>, _> = sessions.collect();
        result.map_err(|e| Error::DatabaseError(e.to_string()))
    }

    /// Get a single session
    pub fn get_session(&self, session_id: &str) -> Result<Option<Session>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Get the last completed session for a project
    pub fn get_last_completed_session(&self, project_id: &str) -> Result<Option<Session>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE project_id = ?1 AND ended_at IS NOT NULL ORDER BY ended_at DESC LIMIT 1",
        )?;

        let result = stmt.query_row(params![project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Delete a session and all its logs/metrics
    pub fn delete_session(&self, session_id: &str) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM metrics WHERE session_id = ?1",
            params![session_id],
        )?;
        self.conn.execute(
            "DELETE FROM logs WHERE session_id = ?1",
            params![session_id],
        )?;
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;

        Ok(())
    }

    /// Get the current (active) session for a project
    pub fn get_current_session_for_project(
        &self,
        project_id: &str,
    ) -> Result<Option<String>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM sessions WHERE project_id = ?1 AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
        )?;

        let result: Result<String, _> = stmt.query_row(params![project_id], |row| row.get(0));

        match result {
            Ok(session_id) => Ok(Some(session_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    // ============================================================================
    // Log Operations
    // ============================================================================

    /// Insert a log entry
    pub fn insert_log(&self, session_id: &str, stream: LogStream, data: &str) -> Result<(), Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let stream_str = match stream {
            LogStream::Stdout => "stdout",
            LogStream::Stderr => "stderr",
        };

        self.conn.execute(
            "INSERT INTO logs (session_id, stream, data, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, stream_str, data, now],
        )?;

        Ok(())
    }

    /// Get logs for a session as text
    pub fn get_session_logs(&self, session_id: &str) -> Result<String, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM logs WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;

        let mut result = String::new();
        for data in rows.flatten() {
            result.push_str(&data);
        }

        Ok(result)
    }

    /// Get logs for the latest session of a project
    pub fn get_project_logs(&self, project_id: &str) -> Result<String, Error> {
        // Get latest session
        let session_id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1",
                params![project_id],
                |row| row.get(0),
            )
            .ok();

        match session_id {
            Some(sid) => self.get_session_logs(&sid),
            None => Ok(String::new()),
        }
    }

    /// Get recent logs (limited count) for the latest session
    pub fn get_recent_logs(&self, project_id: &str, limit: u32) -> Result<String, Error> {
        // Get latest session
        let session_id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1",
                params![project_id],
                |row| row.get(0),
            )
            .ok();

        match session_id {
            Some(sid) => {
                let mut stmt = self.conn.prepare(
                    "SELECT data FROM (
                        SELECT data, timestamp, id FROM logs 
                        WHERE session_id = ?1 
                        ORDER BY timestamp DESC, id DESC LIMIT ?2
                    ) sub 
                    ORDER BY timestamp ASC, id ASC",
                )?;

                let rows = stmt.query_map(params![sid, limit], |row| row.get::<_, String>(0))?;

                let mut result = String::new();
                for data in rows.flatten() {
                    result.push_str(&data);
                }
                Ok(result)
            }
            None => Ok(String::new()),
        }
    }

    /// Clear all logs for a project
    pub fn clear_project_logs(&self, project_id: &str) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE project_id = ?1)",
            params![project_id],
        )?;

        Ok(())
    }

    // ============================================================================
    // Metric Operations
    // ============================================================================

    /// Insert a metric point
    pub fn insert_metric(
        &self,
        session_id: &str,
        cpu_usage: f32,
        memory_usage: u64,
    ) -> Result<(), Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.conn.execute(
            "INSERT INTO metrics (session_id, cpu_usage, memory_usage, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, cpu_usage, memory_usage, now],
        )?;

        Ok(())
    }

    /// Get all metrics for a session
    pub fn get_session_metrics(&self, session_id: &str) -> Result<Vec<MetricPoint>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1 ORDER BY timestamp ASC"
        )?;

        let metrics = stmt.query_map(params![session_id], |row| {
            Ok(MetricPoint {
                cpu_usage: row.get(0)?,
                memory_usage: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })?;

        let result: Result<Vec<_>, _> = metrics.collect();
        result.map_err(|e| Error::DatabaseError(e.to_string()))
    }

    /// Get the last metric for a session
    pub fn get_last_metric(&self, session_id: &str) -> Result<Option<MetricPoint>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT 1"
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            Ok(MetricPoint {
                cpu_usage: row.get(0)?,
                memory_usage: row.get(1)?,
                timestamp: row.get(2)?,
            })
        });

        match result {
            Ok(metric) => Ok(Some(metric)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    // ============================================================================
    // Maintenance Operations
    // ============================================================================

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> Result<(u64, u64, u64, u64), Error> {
        let session_count: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

        let log_count: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM logs", [], |row| row.get(0))?;

        let log_size: u64 = self.conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM logs",
            [],
            |row| row.get(0),
        )?;

        let metric_count: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM metrics", [], |row| row.get(0))?;

        Ok((session_count, log_count, log_size, metric_count))
    }

    /// Clean up old data (sessions older than specified days)
    pub fn cleanup_old_data(&self, days_to_keep: u32) -> Result<(), Error> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - (days_to_keep as u64 * 24 * 60 * 60 * 1000);

        // Delete old sessions (cascade will delete logs and metrics)
        self.conn.execute(
            "DELETE FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?1",
            params![cutoff],
        )?;

        Ok(())
    }

    /// Clean up all data
    pub fn cleanup_all_data(&self) -> Result<(), Error> {
        self.conn.execute("DELETE FROM metrics", [])?;
        self.conn.execute("DELETE FROM logs", [])?;
        self.conn.execute("DELETE FROM sessions", [])?;

        Ok(())
    }

    /// Vacuum the database to reclaim space
    pub fn vacuum(&self) -> Result<(), Error> {
        self.conn.execute("VACUUM", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_group() -> Group {
        Group {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Group".to_string(),
            directory: "/test/dir".to_string(),
            projects: vec![],
            env_vars: HashMap::new(),
            sync_file: None,
            sync_enabled: false,
        }
    }

    #[test]
    fn test_create_and_get_group() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let group = create_test_group();
        db.create_group(&group).unwrap();

        let retrieved = db.get_group(&group.id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, group.name);
        assert_eq!(retrieved.directory, group.directory);
    }

    #[test]
    fn test_delete_group() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let group = create_test_group();
        db.create_group(&group).unwrap();

        db.delete_group(&group.id).unwrap();

        let retrieved = db.get_group(&group.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_settings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let settings = AppSettings {
            max_log_lines: 5000,
            editor: Some("code".to_string()),
        };

        db.update_settings(&settings).unwrap();

        let retrieved = db.get_settings().unwrap();
        assert_eq!(retrieved.max_log_lines, 5000);
        assert_eq!(retrieved.editor, Some("code".to_string()));
    }

    #[test]
    fn test_session_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        // Create a group and project first
        let mut group = create_test_group();
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Project".to_string(),
            command: "echo test".to_string(),
            auto_restart: false,
            cwd: None,
            project_type: ProjectType::Service,
            interactive: false,
            env_vars: HashMap::new(),
        };
        group.projects.push(project.clone());
        db.create_group(&group).unwrap();

        // Create session
        let session_id = db.create_session(&project.id).unwrap();

        // Get session
        let session = db.get_session(&session_id).unwrap();
        assert!(session.is_some());

        // End session
        db.end_session(&session_id, "0").unwrap();

        // Verify ended
        let session = db.get_session(&session_id).unwrap().unwrap();
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn test_logs() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        // Create a group and project first
        let mut group = create_test_group();
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Project".to_string(),
            command: "echo test".to_string(),
            auto_restart: false,
            cwd: None,
            project_type: ProjectType::Service,
            interactive: false,
            env_vars: HashMap::new(),
        };
        group.projects.push(project.clone());
        db.create_group(&group).unwrap();

        let session_id = db.create_session(&project.id).unwrap();

        // Insert logs
        db.insert_log(&session_id, LogStream::Stdout, "Hello ")
            .unwrap();
        db.insert_log(&session_id, LogStream::Stdout, "World")
            .unwrap();

        // Get logs
        let logs = db.get_session_logs(&session_id).unwrap();
        assert_eq!(logs, "Hello World");
    }

    #[test]
    fn test_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        // Create a group and project first
        let mut group = create_test_group();
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Project".to_string(),
            command: "echo test".to_string(),
            auto_restart: false,
            cwd: None,
            project_type: ProjectType::Service,
            interactive: false,
            env_vars: HashMap::new(),
        };
        group.projects.push(project.clone());
        db.create_group(&group).unwrap();

        let session_id = db.create_session(&project.id).unwrap();

        // Insert metrics
        db.insert_metric(&session_id, 10.5, 1024).unwrap();
        db.insert_metric(&session_id, 20.5, 2048).unwrap();

        // Get metrics
        let metrics = db.get_session_metrics(&session_id).unwrap();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].cpu_usage, 10.5);
        assert_eq!(metrics[1].memory_usage, 2048);
    }
}
