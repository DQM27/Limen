// Sin esto, en Windows se abriría una ventana de consola junto a la app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    limen_escritorio_lib::run();
}
