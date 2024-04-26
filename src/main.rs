use log::error;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

mod cli;
fn canonicalize(path: Option<PathBuf>) -> PathBuf {
    let p = path.unwrap_or(PathBuf::from("."));
    p.canonicalize().unwrap_or(p)
}
fn get_filter_file<P: AsRef<Path>>(prefix: P, filter: Option<PathBuf>) -> File {
    let path = if let Some(filter) = filter {
        filter
    } else {
        prefix.as_ref().join("./.obfsct")
    };
    if let Ok(metadata) = path.metadata() {
        if !metadata.is_file() {
            error!("指定的规则文件路径不是文件！");
            panic!()
        }
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).read(true).write(true);
    options.open(path).unwrap()
}
fn list_dir_entries<P: AsRef<Path>>(root: P) -> (Vec<ignore::DirEntry>, Vec<ignore::DirEntry>) {
    let entries = ignore::WalkBuilder::new(root.as_ref())
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.path() != root.as_ref());
    let (files, mut dirs): (Vec<_>, _) = entries.partition(|e| e.path().is_file());
    dirs.sort_by_key(|a| a.path().as_os_str().len());
    (files, dirs)
}
fn get_names_map<P: AsRef<Path>>(prefix: P, filter: &mut File) -> HashMap<PathBuf, String> {
    let mut s = String::new();
    filter.read_to_string(&mut s).unwrap_or_else(|e| {
        error!("读取规则文件时失败，错误：{e}");
        panic!()
    });
    let r: HashMap<PathBuf, _> = toml::from_str(&s).unwrap_or_else(|e| {
        error!("解析规则文件时失败，错误：{e}");
        panic!()
    });
    r.into_iter()
        .map(|(k, v)| (prefix.as_ref().join(k), v))
        .collect()
}
fn obfuscate<P: AsRef<Path>>(
    prefix: P,
    names_map: HashMap<PathBuf, String>,
) -> HashMap<PathBuf, String> {
    let mut new_map = HashMap::new();
    let (files, mut dirs): (Vec<_>, _) = names_map.into_iter().partition(|(k, _)| k.is_file());
    dirs.sort_by_key(|(k, _)| k.as_os_str().len());
    for (mut p, uuid) in dirs.into_iter().chain(files.into_iter()) {
        let file_name = p.clone();
        p.set_file_name(&uuid);
        if let Some(ext) = file_name.extension() {
            p.set_extension(ext);
        }
        fs::rename(&p, &file_name).unwrap_or_else(|e| {
            error!("重命名时发生错误：{e}, {p:?}");
            new_map.insert(file_name, uuid);
        });
    }
    new_map
        .into_iter()
        .map(|(k, v)| (prefix.as_ref().join(k), v))
        .collect()
}
fn main() {
    let env = env_logger::Env::default().filter_or("RUST_LOG", "info");
    let mut builder = env_logger::Builder::from_env(env);
    builder.target(env_logger::Target::Stdout);
    builder.init();
    let cli::Args {
        deobfuscate,
        filter,
        root,
    } = clap::Parser::parse();
    let root = canonicalize(root);
    let root = root.into_os_string().into_string().unwrap_or_else(|e| {
        error!(
            "工作目录{:?}，存在非 UTF-8 字符。无法继续，退出。",
            e.to_string_lossy()
        );
        panic!()
    });
    let mut filter = get_filter_file(&root, filter);
    println!("root: {root:?}");
    let old_map = get_names_map(&root, &mut filter);
    if deobfuscate {
        let new_map = obfuscate(&root, old_map);
        filter.set_len(0).unwrap();
        let new_map = toml::to_string_pretty(&new_map).unwrap();
        filter.write_all(new_map.as_bytes()).unwrap()
    } else {
        let (files, dirs) = list_dir_entries(&root);
        let mut map = HashMap::new();
        for entry in files
            .into_iter()
            .chain(dirs.into_iter().rev())
            .filter(|e| !old_map.contains_key(e.path()))
        {
            println!("{entry:?}");
            let uuid = uuid::Uuid::new_v4().to_string();
            let full_path = entry.into_path();
            let ext = full_path.extension();
            let mut renamed_path = full_path.with_file_name(&uuid);
            if let Some(ext) = ext {
                renamed_path.set_extension(ext);
            }
            fs::rename(&full_path, &renamed_path).unwrap();
            let full_path = full_path.strip_prefix(&root).unwrap().to_path_buf();
            map.insert(full_path, uuid);
        }
        let map = toml::to_string_pretty(&map).unwrap();
        println!("{map}");
        filter.write_all(b"").unwrap();
        filter.write_all(map.as_bytes()).unwrap();
    }
}
