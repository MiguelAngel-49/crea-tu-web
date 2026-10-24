mod commands;
mod models;
mod paths;

use commands::{docker, hosts, sites};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            docker::check_docker,
            hosts::get_hosts_info,
            hosts::add_hosts_entry,
            hosts::remove_hosts_entry,
            sites::list_sites,
            sites::create_site,
            sites::start_site,
            sites::stop_site,
            sites::delete_site,
            sites::get_site_url,
            sites::get_site_admin_url,
            sites::get_data_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar Crea-tu-Web");
}
