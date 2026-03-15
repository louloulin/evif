// CLI 定义

use crate::commands::EvifCommand;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "evif")]
#[command(about = "EVIF - Everything Is a File command-line tool", long_about = None)]
#[command(version)]
pub struct EvifCli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Server address (REST base URL, e.g. http://localhost:8081)
    #[arg(short, long, global = true, default_value = "http://localhost:8081")]
    pub server: String,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Query the graph
    Query {
        /// Query string
        query: String,
    },

    /// List directory contents
    Ls {
        /// Directory path
        #[arg(default_value = "/")]
        path: String,
    },

    /// Display file contents
    Cat {
        /// File path
        path: String,
    },

    /// Write content to file (create or overwrite; use -a to append)
    Write {
        /// File path
        path: String,
        /// Content to write (default: empty)
        #[arg(short, long)]
        content: Option<String>,
        /// Append instead of overwrite
        #[arg(short, long)]
        append: bool,
    },

    /// Create directory
    Mkdir {
        /// Directory path
        path: String,
        /// Create parent directories
        #[arg(short, long)]
        parents: bool,
    },

    /// Remove file or directory
    Rm {
        /// Path to remove
        path: String,
        /// Remove recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Move or rename file/directory
    Mv {
        /// Source path
        src: String,
        /// Destination path
        dst: String,
    },

    /// Show file or directory status
    Stat {
        /// Path to file or directory
        path: String,
    },

    /// Create empty file or update mtime
    Touch {
        /// Path to file
        path: String,
    },

    /// Check server health
    Health,

    /// Search file contents (grep)
    Grep {
        /// Path to search (directory or file)
        path: String,
        /// Pattern to search
        pattern: String,
        /// Search recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Compute file digest/checksum
    Digest {
        /// Path to file
        path: String,
        /// Algorithm (sha256, md5, etc.; default sha256)
        #[arg(short, long, default_value = "sha256")]
        algorithm: String,
    },

    /// Show first N lines of file
    Head {
        /// File path
        path: String,
        /// Number of lines (default: 10)
        #[arg(short, long, default_value = "10")]
        lines: usize,
    },

    /// Show last N lines of file
    Tail {
        /// File path
        path: String,
        /// Number of lines (default: 10)
        #[arg(short, long, default_value = "10")]
        lines: usize,
    },

    /// Show directory tree
    Tree {
        /// Directory path (default: /)
        #[arg(default_value = "/")]
        path: String,
        /// Max depth (default: 3)
        #[arg(short, long, default_value = "3")]
        depth: usize,
    },

    /// Copy files
    Cp {
        /// Source path
        src: String,
        /// Destination path
        dst: String,
    },

    /// Display statistics
    Stats,

    /// Start interactive REPL
    Repl,

    /// Get node by ID
    Get {
        /// Node ID
        id: String,
    },

    /// Create a new node
    Create {
        /// Node type (file, directory, device)
        #[arg(short, long)]
        node_type: String,

        /// Node name
        name: String,

        /// Parent ID (optional)
        #[arg(short, long)]
        parent: Option<String>,
    },

    /// Delete a node
    Delete {
        /// Node ID
        id: String,
    },

    /// Mount EVIF as a FUSE filesystem
    Mount {
        /// Mount point path
        mount_point: String,

        /// Allow write operations
        #[arg(short, long)]
        write: bool,

        /// Cache size (number of inodes)
        #[arg(short, long, default_value = "10000")]
        cache_size: usize,

        /// Cache timeout in seconds
        #[arg(short, long, default_value = "60")]
        cache_timeout: u64,
    },

    /// Unmount a FUSE filesystem
    Umount {
        /// Mount point path
        mount_point: String,
    },

    /// List all mount points (REST API)
    ListMounts,

    /// Mount plugin at path via REST API (e.g. mem at /mem2)
    MountPlugin {
        /// Plugin name (mem, hello, local)
        plugin: String,
        /// Mount path (e.g. /mem2)
        path: String,
        /// Optional config (e.g. root=/tmp for local)
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Unmount plugin at path via REST API
    UnmountPlugin {
        /// Mount path to unmount (e.g. /mem2)
        path: String,
    },

    /// Upload local file to EVIF
    Upload {
        /// Local file path
        local_path: String,
        /// Remote path in EVIF
        remote_path: String,
    },

    /// Download file from EVIF to local
    Download {
        /// Remote path in EVIF
        remote_path: String,
        /// Local file path
        local_path: String,
    },

    /// Display a line of text
    Echo {
        /// Text to display
        text: String,
    },

    /// Change working directory
    Cd {
        /// Directory path
        path: String,
    },

    /// Print working directory
    Pwd,

    /// Sort lines of text
    Sort {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
        /// Reverse sort
        #[arg(short, long)]
        reverse: bool,
        /// Numeric sort
        #[arg(short, long)]
        numeric: bool,
        /// Unique lines
        #[arg(short, long)]
        unique: bool,
    },

    /// Report or omit repeated lines
    Uniq {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
        /// Count occurrences
        #[arg(short, long)]
        count: bool,
    },

    /// Print newline, word, and byte counts
    Wc {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
        /// Print line count
        #[arg(short, long)]
        lines: bool,
        /// Print word count
        #[arg(short, long)]
        words: bool,
        /// Print byte count
        #[arg(short, long)]
        bytes: bool,
    },

    /// Print current date and time
    Date {
        /// Format string (RFC 3339 by default)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Delay for a specified amount of time
    Sleep {
        /// Number of seconds to sleep
        seconds: u64,
    },

    /// Display file differences
    Diff {
        /// First file path
        path1: String,
        /// Second file path
        path2: String,
    },

    /// Estimate file space usage
    Du {
        /// Directory path
        #[arg(default_value = "/")]
        path: String,
        /// Summarize only
        #[arg(short, long)]
        summarize: bool,
    },

    /// Remove sections from each line of files
    Cut {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
        /// Select only these bytes (comma-separated, e.g., 1-5,8)
        #[arg(short = 'b', long)]
        bytes: Option<String>,
        /// Select only these characters (comma-separated, e.g., 1-5,8)
        #[arg(short = 'c', long)]
        chars: Option<String>,
        /// Select only these fields (comma-separated, e.g., 1,3,5)
        #[arg(short = 'f', long)]
        fields: Option<String>,
        /// Field delimiter (default: TAB)
        #[arg(short = 'd', long)]
        delimiter: Option<String>,
    },

    /// Translate or delete characters
    Tr {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
        /// Characters to translate from
        #[arg(short, long)]
        from: String,
        /// Characters to translate to
        #[arg(short, long)]
        to: String,
        /// Delete characters instead of translating
        #[arg(short, long)]
        delete: bool,
    },

    /// Encode/Decode data in base64
    Base {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
        /// Decode instead of encode
        #[arg(short, long)]
        decode: bool,
    },

    /// Print environment variables
    Env,

    /// Export environment variable
    Export {
        /// Variable name and value (e.g., NAME=value)
        #[arg(value_name = "NAME=value")]
        variable: String,
    },

    /// Unset environment variable
    Unset {
        /// Variable name
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Return true (exit code 0)
    True,

    /// Return false (exit code 1)
    False,

    /// Print basename of file path
    Basename {
        /// Path
        path: String,
    },

    /// Print dirname of file path
    Dirname {
        /// Path
        path: String,
    },

    /// Create links between files
    Ln {
        /// Target path
        target: String,
        /// Link name
        link_name: String,
        /// Create symbolic link instead of hard link
        #[arg(short, long)]
        symbolic: bool,
    },

    /// Read symbolic link target
    Readlink {
        /// Link path
        path: String,
    },

    /// Realpath - print resolved path
    Realpath {
        /// Path
        #[arg(default_value = ".")]
        path: String,
    },

    /// Concatenate and print files in reverse
    Rev {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Reverse lines of file
    Tac {
        /// File path (optional, reads from stdin if not provided)
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Truncate file to specified size
    Truncate {
        /// File path
        path: String,
        /// Size in bytes (0 to clear)
        #[arg(short, long, default_value = "0")]
        size: u64,
    },

    /// Split file into pieces
    Split {
        /// File path
        #[arg(short, long)]
        file: Option<String>,
        /// Line count per split
        #[arg(short = 'l', long)]
        lines: Option<usize>,
    },

    /// Find files by name
    Find {
        /// Start directory
        #[arg(default_value = ".")]
        path: String,
        /// File name pattern
        #[arg(short, long)]
        name: Option<String>,
        /// File type (f=file, d=directory)
        #[arg(short, long)]
        type_: Option<String>,
    },

    /// Locate files by name (search database)
    Locate {
        /// File name pattern
        pattern: String,
    },

    /// Locate a command
    Which {
        /// Command name
        command: String,
    },

    /// Display command type
    Type {
        /// Command name
        command: String,
    },

    /// Display file type
    File {
        /// File path
        path: String,
    },
}

impl EvifCli {
    pub async fn run(&self) -> anyhow::Result<()> {
        let command = EvifCommand::new(self.server.clone(), self.verbose);

        match &self.command {
            Commands::Query { query } => {
                command.query(query.clone()).await?;
            }
            Commands::Ls { path } => {
                command.ls(Some(path.clone()), false, false).await?;
            }
            Commands::Cat { path } => {
                command.cat(path.clone()).await?;
            }
            Commands::Write {
                path,
                content,
                append,
            } => {
                command
                    .write(path.clone(), content.clone().unwrap_or_default(), *append)
                    .await?;
            }
            Commands::Mkdir { path, parents } => {
                command.mkdir(path.clone(), *parents).await?;
            }
            Commands::Rm { path, recursive } => {
                command.rm(path.clone(), *recursive).await?;
            }
            Commands::Mv { src, dst } => {
                command.mv(src.clone(), dst.clone()).await?;
            }
            Commands::Stat { path } => {
                command.stat(path.clone()).await?;
            }
            Commands::Touch { path } => {
                command.touch(path.clone()).await?;
            }
            Commands::Health => {
                command.health().await?;
            }
            Commands::Grep {
                path,
                pattern,
                recursive,
            } => {
                command
                    .grep(path.clone(), pattern.clone(), *recursive)
                    .await?;
            }
            Commands::Digest { path, algorithm } => {
                command.checksum(path.clone(), algorithm.clone()).await?;
            }
            Commands::Head { path, lines } => {
                command.head(path.clone(), *lines).await?;
            }
            Commands::Tail { path, lines } => {
                command.tail(path.clone(), *lines).await?;
            }
            Commands::Tree { path, depth } => {
                command.tree(path.clone(), *depth, *depth).await?;
            }
            Commands::Cp { src, dst } => {
                command.cp(src.clone(), dst.clone()).await?;
            }
            Commands::Stats => {
                command.stats().await?;
            }
            Commands::Repl => {
                command.repl().await?;
            }
            Commands::Get { id } => {
                command.get(id.clone()).await?;
            }
            Commands::Create {
                node_type,
                name,
                parent,
            } => {
                command
                    .create(node_type.clone(), name.clone(), parent.clone())
                    .await?;
            }
            Commands::Delete { id } => {
                command.delete(id.clone()).await?;
            }
            Commands::Mount {
                mount_point,
                write,
                cache_size,
                cache_timeout,
            } => {
                let config = Some(format!(
                    "write={},cache_size={},cache_timeout={}",
                    write, cache_size, cache_timeout
                ));
                command
                    .mount("fuse".to_string(), mount_point.clone(), config)
                    .await?;
            }
            Commands::Umount { mount_point } => {
                command.unmount(mount_point.clone()).await?;
            }
            Commands::ListMounts => {
                command.mounts().await?;
            }
            Commands::MountPlugin {
                plugin,
                path,
                config,
            } => {
                command
                    .mount(plugin.clone(), path.clone(), config.clone())
                    .await?;
            }
            Commands::UnmountPlugin { path } => {
                command.unmount(path.clone()).await?;
            }
            Commands::Upload {
                local_path,
                remote_path,
            } => {
                command
                    .upload(local_path.clone(), remote_path.clone())
                    .await?;
            }
            Commands::Download {
                remote_path,
                local_path,
            } => {
                command
                    .download(remote_path.clone(), local_path.clone())
                    .await?;
            }
            Commands::Echo { text } => {
                command.echo(text.clone()).await?;
            }
            Commands::Cd { path } => {
                command.cd(path.clone()).await?;
            }
            Commands::Pwd => {
                command.pwd().await?;
            }
            Commands::Sort {
                file,
                reverse,
                numeric,
                unique,
            } => {
                command
                    .sort(file.clone(), *reverse, *numeric, *unique)
                    .await?;
            }
            Commands::Uniq { file, count } => {
                command.uniq(file.clone(), *count).await?;
            }
            Commands::Wc {
                file,
                lines,
                words,
                bytes,
            } => {
                command.wc(file.clone(), *lines, *words, *bytes).await?;
            }
            Commands::Date { format } => {
                command.date(format.clone()).await?;
            }
            Commands::Sleep { seconds } => {
                command.sleep(*seconds).await?;
            }
            Commands::Diff { path1, path2 } => {
                command.diff(path1.clone(), path2.clone()).await?;
            }
            Commands::Du { path, summarize } => {
                command.du(path.clone(), !*summarize).await?;
            }
            Commands::Cut {
                file,
                bytes,
                chars,
                fields,
                delimiter,
            } => {
                command
                    .cut(
                        file.clone(),
                        bytes.clone(),
                        chars.clone(),
                        fields.clone(),
                        delimiter.clone(),
                    )
                    .await?;
            }
            Commands::Tr {
                file,
                from,
                to,
                delete,
            } => {
                command
                    .tr_(file.clone(), from.clone(), to.clone(), *delete)
                    .await?;
            }
            Commands::Base { file, decode } => {
                command.base(file.clone(), *decode).await?;
            }
            Commands::Env => {
                command.env().await?;
            }
            Commands::Export { variable } => {
                command.export(variable.clone()).await?;
            }
            Commands::Unset { name } => {
                command.unset(name.clone()).await?;
            }
            Commands::True => {
                command.true_cmd().await?;
            }
            Commands::False => {
                command.false_cmd().await?;
            }
            Commands::Basename { path } => {
                command.basename(path.clone()).await?;
            }
            Commands::Dirname { path } => {
                command.dirname(path.clone()).await?;
            }
            Commands::Ln {
                target,
                link_name,
                symbolic,
            } => {
                command
                    .ln(target.clone(), link_name.clone(), *symbolic)
                    .await?;
            }
            Commands::Readlink { path } => {
                command.readlink(path.clone()).await?;
            }
            Commands::Realpath { path } => {
                command.realpath(path.clone()).await?;
            }
            Commands::Rev { file } => {
                command.rev(file.clone()).await?;
            }
            Commands::Tac { file } => {
                command.tac(file.clone()).await?;
            }
            Commands::Truncate { path, size } => {
                command.truncate(path.clone(), *size).await?;
            }
            Commands::Split { file, lines } => {
                command.split(file.clone(), lines.clone()).await?;
            }
            Commands::Find { path, name, type_ } => {
                command
                    .find(path.clone(), name.as_deref(), type_.as_deref())
                    .await?;
            }
            Commands::Locate { pattern } => {
                command.locate(pattern.clone()).await?;
            }
            Commands::Which { command: cmd } => {
                command.which(cmd.clone()).await?;
            }
            Commands::Type { command: cmd } => {
                command.type_cmd(cmd.clone()).await?;
            }
            Commands::File { path } => {
                command.file(path.clone()).await?;
            }
        }

        Ok(())
    }
}
