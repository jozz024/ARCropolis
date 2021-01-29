use std::{fs, path::{
        Path,
        PathBuf
    }};

use log::warn;
use smash_arc::{Hash40, Region};

use crate::CONFIG;

use crate::replacement_files::FileCtx;

enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

pub struct ModPath(PathBuf);

impl ModPath {
    pub fn new<P: AsRef<Path>>(path: &P) -> Self {
        Self(PathBuf::from(path.as_ref()))
    }

    pub fn as_smash_path(&self) -> PathBuf {
        let mut arc_path = self.0.to_str().unwrap().to_string();

        if let Some(_) = arc_path.find(";") {
            arc_path = arc_path.replace(";", ":");
        }

        if let Some(regional_marker) = arc_path.find("+") {
            arc_path.replace_range(regional_marker..arc_path.find(".").unwrap(), "");
        }

        if let Some(ext) = arc_path.strip_suffix("mp4") {
            arc_path = format!("{}{}", ext, "webm");
        }

        PathBuf::from(arc_path)
    }

    pub fn hash40(&self) -> Result<Hash40, String> {
        let smash_path = self.as_smash_path();

        match smash_path.to_str() {
            Some(path) => Ok(Hash40::from(path)),
            // TODO: Replace this by a proper error. This-error or something else.
            None => Err(String::from(format!("Couldn't convert {} to a &str", self.0.display()))),
        }
    }

    pub fn is_stream(&self) -> bool {
        self.0.starts_with("stream")
    }

    pub fn get_region(&self) -> Option<Region> {
        match self.0.extension() {
            Some(_) => {
                // Split the region identifier from the filepath
                let filename = self.0.file_name().unwrap().to_str().unwrap().to_string();
                // Check if the filepath it contains a + symbol
                let region = if let Some(region_marker) = filename.find('+') {
                    let region = match &filename[region_marker + 1..region_marker + 6] {
                        "jp_ja" => Region::Japanese,
                        "us_en" => Region::UsEnglish,
                        "us_fr" => Region::UsFrench,
                        "us_es" => Region::UsSpanish,
                        "eu_en" => Region::EuEnglish,
                        "eu_fr" => Region::EuFrench,
                        "eu_es" => Region::EuSpanish,
                        "eu_de" => Region::EuGerman,
                        "eu_nl" => Region::EuDutch,
                        "eu_it" => Region::EuItalian,
                        "eu_ru" => Region::EuRussian,
                        "kr_ko" => Region::Korean,
                        "zh_cn" => Region::ChinaChinese,
                        "zh_tw" => Region::TaiwanChinese,
                        // If the regional indicator makes no sense, default to us_en
                        _ => Region::UsEnglish,
                    };

                    Some(region)
                } else {
                    None
                };

                region
            },
            None => None,
        }
    }
}

/// Visit Ultimate Mod Manager directories for backwards compatibility
pub fn umm_directories<P: AsRef<Path>>(path: &P) -> Vec<FileCtx> {
    let mut vec = Vec::new();
    let base_path = path.as_ref();

    // TODO: Careful here, sometimes a /umm path does not exist.
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();

        // Skip any directory starting with a period
        if entry.file_name().to_str().unwrap().starts_with(".") {
            continue;
        }

        let mut subdir_path = base_path.to_path_buf();
        subdir_path.push(entry.path());
        
        vec.append(&mut directory(&subdir_path));
    }

    vec
}

pub fn directory<P: AsRef<Path>>(path: &P) -> Vec<FileCtx> {
    let path = path.as_ref();

    // TODO: Make sure the path exists before proceeding
    let paths: Vec<OneOrMany<FileCtx>> = fs::read_dir(path).unwrap().filter_map(|entry| {
        let entry = entry.unwrap();

        let mut entry_path = path.to_path_buf();
        entry_path.push(entry.path());

        if entry_path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            return None;
        }

        if entry.file_type().unwrap().is_dir() {
            return Some(OneOrMany::Many(directory(&entry_path)));
        } else {
            match file(&entry_path) {
                Ok(file_ctx) => {
                    return Some(OneOrMany::One(file_ctx));
                },
                Err(err) => {
                    warn!("{}", err);
                    return None;
                }
            }
        }
    }).collect();

    let mut final_vec: Vec<FileCtx> = Vec::new();

    for instance in paths {
        match instance {
            OneOrMany::One(context) => final_vec.push(context),
            OneOrMany::Many(mut contexts) => final_vec.append(&mut contexts),
        }
    }

    final_vec
}

pub fn file<P: AsRef<Path>>(path: &P) -> Result<FileCtx, String> {
    let path = path.as_ref();

        if path.is_dir() {
            return Err("[ARC::Discovery] Not a file".to_string());
        }

        warn!("[ARC::Discovery] File '{}'", path.display());

        // Skip any file starting with a period, to avoid any error related to path.extension()
        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            return Err(format!("[ARC::Discovery] File '{}' starts with a period, skipping", path.display()));
        }

        // Make sure the file has an extension to not cause issues with the code that follows
        if path.extension() == None {
            return Err(format!("[ARC::Discovery] File '{}' does not have an extension, skipping", path.display()));
        }

        // let mut arc_path = match path.strip_prefix(path) {
        //     Ok(stripped_path) => String::from(stripped_path.to_str().unwrap()),
        //     Err(_) => return None,
        // };

        let mut arc_path = path.to_str().unwrap().to_string();

        if let Some(_) = arc_path.find(";") {
            arc_path = arc_path.replace(";", ":");
        }

        if let Some(regional_marker) = arc_path.find("+") {
            // TODO: Return here if the region doesn't match the game's
            arc_path.replace_range(regional_marker..arc_path.find(".").unwrap(), "");
        }

        // TODO: Move that stuff in a separate function that can handle more than one format
        // TODO: Have it just replace the extension to hash in FileCtx
        if let Some(ext) = arc_path.strip_suffix("mp4") {
            arc_path = format!("{}{}", ext, "webm");
        }

        // TODO: Rework the following atrocity

        let mut file_ctx = FileCtx::new();

        file_ctx.path = path.to_path_buf();
        file_ctx.hash = Hash40::from(arc_path.as_str());
        let ext = Path::new(&arc_path).extension().unwrap().to_str().unwrap();
        file_ctx.extension = Hash40::from(ext);

        file_ctx.filesize = match path.metadata() {
            Ok(meta) => meta.len() as u32,
            Err(err) => panic!(err),
        };

        // TODO: Move this to the regional marker check
        if file_ctx.get_region() != crate::replacement_files::get_region_id(&CONFIG.read().misc.region.as_ref().unwrap()) {
            return Err(format!("[ARC::Discovery] File '{}' does not have a matching region, skipping", file_ctx.path.display()));
        }

        Ok(file_ctx)
}