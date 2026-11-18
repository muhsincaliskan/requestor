const ENVIRONMENT_PREFIX: &str = "environment:";
const PROXY_PASSWORD: &str = "proxy-password";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultSlot(String);

impl VaultSlot {
    pub fn environment(id: &str) -> Self {
        VaultSlot(format!("{ENVIRONMENT_PREFIX}{id}"))
    }

    pub fn proxy_password() -> Self {
        VaultSlot(PROXY_PASSWORD.to_string())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}
