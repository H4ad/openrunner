/**
 * Format CPU usage as a percentage string
 * Returns empty string if usage is 0 or negative
 */
export function formatCpu(usage: number): string {
  return usage > 0 ? `${usage.toFixed(1)}%` : "";
}

/**
 * Format bytes to human-readable size (B, KB, MB, GB)
 * @param bytes - Size in bytes
 * @returns Formatted string with unit
 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Format memory in MB to human-readable size (MB, GB)
 * Used when memory is already in megabytes
 * @param mb - Size in megabytes
 * @returns Formatted string with unit
 */
export function formatMemoryMB(mb: number): string {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)}GB`;
  }
  return mb > 0 ? `${Math.round(mb)}MB` : "";
}
