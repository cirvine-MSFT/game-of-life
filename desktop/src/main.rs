// Suppress the console window on Windows release builds; debug builds keep
// it so panic output remains visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    game_of_life_desktop_lib::run();
}
