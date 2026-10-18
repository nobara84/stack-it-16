use std::{env, fs, path::PathBuf};

const APP_DIRECTORY: &str = "stack-it-16";
const HIGH_SCORE_FILE: &str = "highscore.txt";

pub fn load_high_score() -> u32 {
    let path = high_score_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

pub fn save_high_score(score: u32) {
    let path = high_score_path();
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_ok() {
        let _ = fs::write(path, score.to_string());
    }
}

fn high_score_path() -> PathBuf {
    if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg_data_home)
            .join(APP_DIRECTORY)
            .join(HIGH_SCORE_FILE);
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(APP_DIRECTORY)
            .join(HIGH_SCORE_FILE);
    }

    PathBuf::from(HIGH_SCORE_FILE)
}
