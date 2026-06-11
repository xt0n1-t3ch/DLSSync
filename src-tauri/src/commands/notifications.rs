use crate::error::{AppError, AppResult};
use crate::state::AppState;
use notifications_store::{ListFilter, NotificationEntry, NOTIFICATION_PUSHED_EVENT};
use tauri::{AppHandle, Emitter, State};

fn store_required<'a>(
    guard: &'a parking_lot::RwLockReadGuard<'a, Option<notifications_store::NotificationsStore>>,
) -> AppResult<&'a notifications_store::NotificationsStore> {
    guard
        .as_ref()
        .ok_or_else(|| AppError::Other("notifications store not initialized".into()))
}

#[tauri::command]
pub async fn list_notifications(
    state: State<'_, AppState>,
    filter: Option<ListFilter>,
) -> AppResult<Vec<NotificationEntry>> {
    let guard = state.notifications.read();
    let store = store_required(&guard)?;
    let f = filter.unwrap_or_default();
    Ok(store.list(&f)?)
}

#[tauri::command]
pub async fn mark_notification_read(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let guard = state.notifications.read();
    let store = store_required(&guard)?;
    store.mark_read(&id)?;
    Ok(())
}

#[tauri::command]
pub async fn mark_all_notifications_read(state: State<'_, AppState>) -> AppResult<u32> {
    let guard = state.notifications.read();
    let store = store_required(&guard)?;
    Ok(store.mark_all_read()?)
}

#[tauri::command]
pub async fn dismiss_notification(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let guard = state.notifications.read();
    let store = store_required(&guard)?;
    store.dismiss(&id)?;
    Ok(())
}

const ALLOWED_LINK_HOST_SUFFIXES: &[&str] = &[
    "github.com",
    "pcgamingwiki.com",
    "nvidia.com",
    "amd.com",
    "intel.com",
    "ko-fi.com",
    "nexusmods.com",
];

/// A notification `link` is either an internal route token (e.g. `library`) or an
/// external https URL. A webview-injected push must not be able to render a
/// `javascript:`/`file:`/`http:` or off-allowlist phishing link as a trusted
/// in-app notification, so anything else is dropped to `None`.
fn sanitize_notification_link(link: Option<String>) -> Option<String> {
    let raw = link?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed.contains(':') && !trimmed.contains('/') {
        let is_route = trimmed
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase())
            && trimmed
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        return is_route.then(|| trimmed.to_string());
    }
    let parsed = url::Url::parse(trimmed).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    let host = match parsed.host() {
        Some(url::Host::Domain(d)) => d.to_ascii_lowercase(),
        _ => return None,
    };
    let allowed = ALLOWED_LINK_HOST_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")));
    allowed.then(|| trimmed.to_string())
}

#[tauri::command]
pub async fn push_notification(
    state: State<'_, AppState>,
    app: AppHandle,
    mut entry: NotificationEntry,
) -> AppResult<()> {
    entry.link = sanitize_notification_link(entry.link.take());
    let inserted = {
        let guard = state.notifications.read();
        let store = store_required(&guard)?;
        store.insert(&entry)?
    };
    if inserted {
        let _ = app.emit(NOTIFICATION_PUSHED_EVENT, &entry);
    }
    Ok(())
}

#[tauri::command]
pub async fn notifications_unread_count(state: State<'_, AppState>) -> AppResult<u32> {
    let guard = state.notifications.read();
    let store = store_required(&guard)?;
    Ok(store.unread_count()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;
    use notifications_store::{NotificationKind, NotificationsStore, MAX_NOTIFICATIONS};
    use tempfile::tempdir;

    fn open_via_paths(dir: &tempfile::TempDir) -> NotificationsStore {
        let paths = AppPaths::from_root(dir.path().join("DLSSync"));
        paths.ensure_dirs().unwrap();
        NotificationsStore::open(paths.notifications_db).unwrap()
    }

    fn entry(title: &str, secs_ago: i64, kind: NotificationKind) -> NotificationEntry {
        NotificationEntry {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            title: title.into(),
            body: Some(format!("body-{title}")),
            created_at: chrono::Utc::now() - chrono::Duration::seconds(secs_ago),
            read_at: None,
            dismissed_at: None,
            apply_id: Some(format!("apply-{title}")),
            game_id: Some(format!("steam-{title}")),
            error_class: None,
            link: None,
            vendor: None,
            dedup_key: None,
        }
    }

    #[test]
    fn event_name_constant_matches_frontend_literal() {
        assert_eq!(NOTIFICATION_PUSHED_EVENT, "notification:pushed");
    }

    #[test]
    fn link_sanitizer_keeps_routes_and_allowlisted_https() {
        use super::sanitize_notification_link as san;
        assert_eq!(san(Some("library".into())), Some("library".into()));
        assert_eq!(san(None), None);
        assert_eq!(
            san(Some(
                "https://github.com/xt0n1-t3ch/DLSSync/releases".into()
            )),
            Some("https://github.com/xt0n1-t3ch/DLSSync/releases".into())
        );
        assert_eq!(
            san(Some("https://www.nvidia.com/drivers".into())),
            Some("https://www.nvidia.com/drivers".into())
        );
    }

    #[test]
    fn link_sanitizer_drops_phishing_and_dangerous_schemes() {
        use super::sanitize_notification_link as san;
        assert_eq!(san(Some("https://evil.example.com/login".into())), None);
        assert_eq!(san(Some("http://github.com/x".into())), None);
        assert_eq!(san(Some("javascript:alert(1)".into())), None);
        assert_eq!(san(Some("file:///c:/windows/system32".into())), None);
        assert_eq!(san(Some("https://github.com.evil.com/x".into())), None);
        assert_eq!(san(Some("../../etc/passwd".into())), None);
    }

    #[test]
    fn store_opens_from_app_paths_layout() {
        let dir = tempdir().unwrap();
        let paths = AppPaths::from_root(dir.path().join("DLSSync"));
        paths.ensure_dirs().unwrap();
        let store = NotificationsStore::open(paths.notifications_db.clone()).unwrap();
        assert_eq!(store.total_count().unwrap(), 0);
        assert!(paths.notifications_db.exists());
    }

    #[test]
    fn end_to_end_push_list_mark_dismiss_flow() {
        let dir = tempdir().unwrap();
        let store = open_via_paths(&dir);

        let a = entry("alpha", 30, NotificationKind::ApplySuccess);
        let b = entry("bravo", 20, NotificationKind::ApplyFailure);
        let c = entry("charlie", 10, NotificationKind::ApplyCancelled);
        store.insert(&a).unwrap();
        store.insert(&b).unwrap();
        store.insert(&c).unwrap();

        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].id, c.id);
        assert_eq!(listed[2].id, a.id);

        assert_eq!(store.unread_count().unwrap(), 3);
        store.mark_read(&b.id).unwrap();
        assert_eq!(store.unread_count().unwrap(), 2);

        let marked_all = store.mark_all_read().unwrap();
        assert_eq!(marked_all, 2);
        assert_eq!(store.unread_count().unwrap(), 0);

        store.dismiss(&a.id).unwrap();
        let after_dismiss = store.list(&ListFilter::default()).unwrap();
        assert_eq!(after_dismiss.len(), 2);
        assert!(after_dismiss.iter().all(|e| e.id != a.id));

        let with_dismissed = store
            .list(&ListFilter {
                include_dismissed: Some(true),
                limit: None,
            })
            .unwrap();
        assert_eq!(with_dismissed.len(), 3);
    }

    #[test]
    fn fifo_eviction_holds_at_cap_under_burst() {
        let dir = tempdir().unwrap();
        let store = open_via_paths(&dir);
        let burst = 250;
        for i in 0..burst {
            let e = entry(
                &format!("burst-{i}"),
                (burst - i) as i64,
                NotificationKind::ApplySuccess,
            );
            store.insert(&e).unwrap();
        }
        assert_eq!(store.total_count().unwrap() as usize, MAX_NOTIFICATIONS);
        let newest = store
            .list(&ListFilter {
                include_dismissed: None,
                limit: Some(1),
            })
            .unwrap();
        assert_eq!(newest[0].title, format!("burst-{}", burst - 1));

        let all_with_dismissed = store
            .list(&ListFilter {
                include_dismissed: Some(true),
                limit: Some(MAX_NOTIFICATIONS as u32 + 100),
            })
            .unwrap();
        assert_eq!(all_with_dismissed.len(), MAX_NOTIFICATIONS);
        let oldest_title = &all_with_dismissed.last().unwrap().title;
        let oldest_idx: usize = oldest_title
            .strip_prefix("burst-")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(oldest_idx, burst - MAX_NOTIFICATIONS);
    }
}
