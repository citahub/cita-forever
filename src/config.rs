#[derive(Debug, Deserialize, Clone, Default)]
pub struct ForeverConfig {
    pub name: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub pidfile: Option<String>,
    pub process: Option<Vec<ProcessConfig>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProcessConfig {
    pub name: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub pidfile: Option<String>,
    pub respawn: Option<u32>,
    pub pid: Option<u32>,
    pub respawns: Option<u32>,
}

impl ForeverConfig {
    pub fn new(path: &str) -> Self {
        parse_config!(ForeverConfig, path)
    }
}
