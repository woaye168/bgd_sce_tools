//! 敏感凭证存储：GitHub Token 存 Windows 凭据管理器（keyring crate，wincred 后端），
//! 不落盘 settings.json。

use anyhow::{Context, Result};

const KEYRING_SERVICE: &str = "bgd_sce_tools";
const KEYRING_USER: &str = "github_token";

/// 读取 GitHub Token（未配置或凭据管理器不可用时返回空串）
pub fn load_github_token() -> String {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .ok()
        .and_then(|entry| entry.get_password().ok())
        .unwrap_or_default()
}

/// 写入 GitHub Token；空串表示删除条目（未找到不算错误）
pub fn save_github_token(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).context("凭据管理器不可用")?;
    if token.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("删除凭据失败: {e}")),
        }
    } else {
        entry.set_password(token).context("写入凭据管理器失败")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        // 防回归：keyring 未启用平台特性时会静默退化为内存 mock（写成功但读不到）。
        // 注意：必须使用独立测试条目——直接操作生产条目会把用户真实 token 清掉
        let entry = keyring::Entry::new("bgd_sce_tools_test", "probe").unwrap();
        eprintln!("credential backend: {entry:?}");
        entry.set_password("probe_xyz").unwrap();
        assert_eq!(entry.get_password().unwrap(), "probe_xyz");
        entry.delete_credential().unwrap();
        assert!(entry.get_password().is_err());
    }
}
