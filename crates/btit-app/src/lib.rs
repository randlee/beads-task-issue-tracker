#[macro_use]
extern crate btit_beads; // brings log_info!/log_warn!/log_error!/log_debug! into textual scope
mod attachment_refs;
mod attachments;
mod backend;
mod config;
mod fs_commands;
mod issue_commands;
mod logging;
mod migration;
mod polling;
mod probe;
#[cfg(test)]
mod test_backend;
mod updates;
mod watcher;

use std::io::Write as _;
use std::sync::Mutex;
use watcher::WatcherState;

// ============================================================================
// App Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env file (dev only — in prod there's no .env, env vars come from the system)
    let _ = dotenvy::dotenv();

    let builder = tauri::Builder::default()
        .manage(Mutex::new(WatcherState::default()))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            logging::install_logging(app)?;

            // Log startup info
            log::info!("=== Beads Task-Issue Tracker starting ===");
            log::info!(
                "[startup] Extended PATH: {}",
                btit_cli::path::get_extended_path()
            );

            // Load config and install the backend for its CLI binary (auto-detects bd→br if no config exists)
            let config = config::load_config();
            log::info!("[startup] CLI binary: {}", config.cli_binary);
            // One `--version` probe (run from the temp dir): selects the backend and feeds the startup log
            let probe = backend::install(&config.cli_binary);
            backend::log_startup(&config.cli_binary, probe.as_ref());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            migration::bd_sync,
            migration::bd_repair_database,
            migration::bd_migrate_to_dolt,
            migration::bd_check_needs_migration,
            migration::bd_cleanup_stale_locks,
            polling::bd_check_changed,
            polling::bd_reset_mtime,
            polling::bd_poll_data,
            issue_commands::bd_list,
            issue_commands::bd_count,
            issue_commands::bd_ready,
            issue_commands::bd_status,
            issue_commands::bd_show,
            issue_commands::bd_create,
            logging::get_logging_enabled,
            logging::set_logging_enabled,
            logging::get_verbose_logging,
            logging::set_verbose_logging,
            logging::clear_logs,
            logging::export_logs,
            logging::read_logs,
            logging::get_log_path_string,
            logging::log_frontend,
            config::get_bd_version,
            backend::check_bd_compatibility,
            config::get_cli_binary_path,
            config::set_cli_binary_path,
            config::validate_cli_binary,
            issue_commands::bd_update,
            issue_commands::bd_close,
            issue_commands::bd_search,
            issue_commands::bd_label_add,
            issue_commands::bd_label_remove,
            issue_commands::bd_delete,
            issue_commands::bd_comments_add,
            issue_commands::bd_dep_add,
            issue_commands::bd_dep_remove,
            issue_commands::bd_dep_add_relation,
            issue_commands::bd_dep_remove_relation,
            issue_commands::bd_available_relation_types,
            fs_commands::fs_exists,
            fs_commands::fs_list,
            updates::check_for_updates,
            updates::check_for_updates_demo,
            updates::check_bd_cli_update,
            updates::download_and_install_update,
            attachments::open_image_file,
            attachments::read_image_file,
            attachments::copy_file_to_attachments,
            attachments::list_attachments,
            attachments::delete_attachment,
            attachments::read_text_file,
            attachments::write_text_file,
            attachments::purge_orphan_attachments,
            attachment_refs::check_refs_migration,
            attachment_refs::migrate_attachment_refs,
            watcher::start_watching,
            watcher::stop_watching,
            watcher::get_watcher_status,
            probe::fetch_external_data,
            probe::check_external_health,
            probe::post_external_data,
            probe::delete_external_data,
            probe::patch_external_data,
            probe::launch_probe,
        ]);
    // `Builder::build` does not run `setup` (Tauri 2.10.2 runs it inside
    // `App::run` on `RuntimeRunEvent::Ready`), so no logger is installed yet in
    // the Err branch: report to stderr only, no `log::error!`.
    match builder.build(tauri::generate_context!()) {
        Ok(app) => app.run(|handle, event| logging::on_run_event(handle, &event)),
        Err(e) => {
            let _ = writeln!(
                std::io::stderr(),
                "beads-task-issue-tracker: failed to build tauri application: {e}"
            );
            std::process::exit(1);
        }
    }
}
