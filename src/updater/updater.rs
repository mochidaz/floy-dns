use crate::config::Config;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const SITES_AVAILABLE_BASE: &str = "/etc/nginx/sites-available";
const SITES_ENABLED_BASE: &str = "/etc/nginx/sites-enabled";

pub(crate) fn get_domain_paths(user_id: &str, business_id: &str) -> (PathBuf, PathBuf) {
    let available = Path::new(SITES_AVAILABLE_BASE)
        .join(user_id)
        .join(business_id)
        .join("nginx.conf");
    let enabled = Path::new(SITES_ENABLED_BASE)
        .join(user_id)
        .join(business_id)
        .join("nginx.conf");
    (available, enabled)
}

pub fn create_domain(
    user_id: &str,
    business_id: &str,
    domain: &str,
    page_id: &str,
    cfg: &Config,
) -> Result<()> {
    let (available_path, enabled_path) = get_domain_paths(user_id, business_id);

    if let Some(dir) = available_path.parent() {
        fs::create_dir_all(dir)?;
    } else {
        return Err(Error::new(ErrorKind::InvalidData, "Path tidak valid"));
    }

    let config_template = format!(
        r#"server {{
    listen 80;
    server_name {}.{};
    root {}/{}/{};
    index index.html;

    location / {{
        rewrite ^/$ /{page_id} break;
    }}
}}
"#,
        domain, cfg.dns_suffix, cfg.prefix, user_id, business_id,
    );

    fs::write(&available_path, config_template)?;

    if !enabled_path.exists() {
        symlink(&available_path, &enabled_path)?;
    }

    Ok(())
}

pub fn delete_domain(user_id: &str, business_id: &str) -> Result<()> {
    let (available_path, enabled_path) = get_domain_paths(user_id, business_id);

    if enabled_path.exists() {
        fs::remove_file(&enabled_path)?;
    }
    if available_path.exists() {
        fs::remove_file(&available_path)?;
    }
    Ok(())
}

fn insert_location_block(content: &mut String, location_block: &str) -> Result<()> {
    if let Some(pos) = content.rfind('}') {
        content.insert_str(pos, location_block);
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::InvalidData,
            "Format file konfigurasi tidak valid",
        ))
    }
}

pub fn add_slug_page(
    user_id: &str,
    business_id: &str,
    slug: &str,
    new_site: &str,
    rewrite_target: Option<&str>,
) -> Result<()> {
    let (available_path, _) = get_domain_paths(user_id, business_id);

    let mut content = fs::read_to_string(&available_path)?;

    let target = rewrite_target.unwrap_or(&format!("/{}/index.html", new_site));

    let location_block = format!(
        r#"
    location /{slug} {{
        rewrite ^/{slug}$ {} break;
    }}
"#,
        slug = slug,
    );

    fs::write(&available_path, content)?;

    Ok(())
}

pub fn update_slug_page(
    user_id: &str,
    business_id: &str,
    slug: &str,
    previous_slug: &str,
    new_site: &str,
    rewrite_target: Option<&str>,
) -> Result<()> {
    let (available_path, _) = get_domain_paths(user_id, business_id);
    let mut content = fs::read_to_string(&available_path)?;
    let start_marker = format!("location /{} {{", previous_slug);

    if let Some(start) = content.find(&start_marker) {
        if let Some(end) = content[start..].find('}') {
            let end_index = start + end + 1;
            let target = rewrite_target.unwrap_or(&format!("/{}/index.html", new_site));
            let new_block = format!(
                r#"location /{slug} {{
        rewrite ^/{slug}$ {} break;
    }}"#,
                slug = slug,
            );
            content.replace_range(start..end_index, &new_block);
            fs::write(&available_path, content)?;
            return Ok(());
        }
    }
    Err(Error::new(ErrorKind::NotFound, "Slug page tidak ditemukan"))
}

pub fn delete_slug_page(user_id: &str, business_id: &str, slug: &str) -> Result<()> {
    let (available_path, _) = get_domain_paths(user_id, business_id);
    let mut content = fs::read_to_string(&available_path)?;
    let start_marker = format!("location /{} {{", slug);

    if let Some(start) = content.find(&start_marker) {
        if let Some(end) = content[start..].find('}') {
            let end_index = start + end + 1;
            content.replace_range(start..end_index, "");
            fs::write(&available_path, content)?;
            return Ok(());
        }
    }
    Err(Error::new(ErrorKind::NotFound, "Slug page tidak ditemukan"))
}
