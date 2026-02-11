use crate::database::config_schema::init_config_database;
use crate::error::Error;
use crate::models::{AppSettings, Group, Project, ProjectType};
use rusqlite::{params, Connection, Transaction};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration database manager
pub struct ConfigDatabase {
    conn: Connection,
    config_dir: PathBuf,
    groups_dir: PathBuf,
}

impl ConfigDatabase {
    /// Initialize or open the configuration database
    pub fn open(config_dir: &Path) -> Result<Self, Error> {
        fs::create_dir_all(config_dir)?;
        let groups_dir = config_dir.join("groups");
        fs::create_dir_all(&groups_dir)?;

        let db_path = config_dir.join("config.db");
        let conn = init_config_database(&db_path)?;

        Ok(Self {
            conn,
            config_dir: config_dir.to_path_buf(),
            groups_dir,
        })
    }

    /// Get the groups directory path
    pub fn groups_dir(&self) -> &Path {
        &self.groups_dir
    }

    /// Get the path for a group's database file
    pub fn group_db_path(&self, group_id: &str) -> PathBuf {
        self.groups_dir.join(format!("{}.db", group_id))
    }

    /// Delete a group's database file
    pub fn delete_group_db(&self, group_id: &str) -> Result<(), Error> {
        let db_path = self.group_db_path(group_id);
        if db_path.exists() {
            fs::remove_file(&db_path)?;
        }
        Ok(())
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
                projects: Vec::new(),     // Will be populated separately
                env_vars: HashMap::new(), // Will be populated separately
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
    pub fn create_group(&self, group: &Group) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

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

    /// Update a group's basic info (not projects or env vars)
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
        // Delete from database (cascade will handle related data)
        self.conn
            .execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;

        // Delete the group's database file
        self.delete_group_db(group_id)?;

        Ok(())
    }

    // ============================================================================
    // Project Operations
    // ============================================================================

    /// Get all projects for a group
    fn get_projects_for_group(&self, group_id: &str) -> Result<Vec<Project>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, auto_restart, cwd, project_type, interactive 
             FROM projects WHERE group_id = ?1 ORDER BY name",
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
                env_vars: HashMap::new(), // Will be populated separately
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
            "SELECT id, name, command, auto_restart, cwd, project_type, interactive 
             FROM projects WHERE id = ?1",
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
            "SELECT p.id, p.name, p.command, p.auto_restart, p.cwd, p.project_type, p.interactive, p.group_id
             FROM projects p WHERE p.id = ?1"
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
    pub fn create_project(&self, group_id: &str, project: &Project) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;
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
            "INSERT INTO projects (id, group_id, name, command, auto_restart, cwd, project_type, interactive) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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
    pub fn update_project(&self, project: &Project) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

        let project_type_str = match project.project_type {
            ProjectType::Task => "task",
            ProjectType::Service => "service",
        };

        tx.execute(
            "UPDATE projects SET name = ?1, command = ?2, auto_restart = ?3, cwd = ?4, project_type = ?5, interactive = ?6 
             WHERE id = ?7",
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
    pub fn delete_projects(&self, project_ids: &[String]) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

        for project_id in project_ids {
            tx.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Convert multiple projects' types in a transaction
    pub fn convert_projects(&self, conversions: &[(String, ProjectType)]) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

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
        &self,
        group_id: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

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
        &self,
        project_id: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

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
    // Sync Operations
    // ============================================================================

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
    pub fn replace_group(&self, group: &Group) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;

        // Update group basic info
        tx.execute(
            "UPDATE groups SET name = ?1, env_vars = ?2 WHERE id = ?3",
            params![group.name, "", group.id], // env_vars stored in separate table
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
        let existing_ids: Vec<String> = {
            let mut stmt = tx.prepare("SELECT id FROM projects WHERE group_id = ?1")?;
            let ids: Vec<String> = stmt
                .query_map(params![group.id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            ids
        };

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
                    "UPDATE projects SET name = ?1, command = ?2, auto_restart = ?3, cwd = ?4, project_type = ?5, interactive = ?6 
                     WHERE id = ?7",
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
        let db = ConfigDatabase::open(temp_dir.path()).unwrap();

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
        let db = ConfigDatabase::open(temp_dir.path()).unwrap();

        let group = create_test_group();
        db.create_group(&group).unwrap();

        db.delete_group(&group.id).unwrap();

        let retrieved = db.get_group(&group.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_settings() {
        let temp_dir = TempDir::new().unwrap();
        let db = ConfigDatabase::open(temp_dir.path()).unwrap();

        let settings = AppSettings {
            max_log_lines: 5000,
            editor: Some("code".to_string()),
        };

        db.update_settings(&settings).unwrap();

        let retrieved = db.get_settings().unwrap();
        assert_eq!(retrieved.max_log_lines, 5000);
        assert_eq!(retrieved.editor, Some("code".to_string()));
    }
}
