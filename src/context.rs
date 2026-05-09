pub struct RuntimeContext {
    pub stdin_is_tty: bool,
    pub stderr_is_tty: bool,
    pub locale: &'static str,
}
