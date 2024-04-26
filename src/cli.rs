use clap::Parser;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(author, version, about = "混淆。", long_about = r#"混淆。"#)]
pub struct Args {
    /// 指定混淆信息保存或加载的位置，默认为工作目录下的 `.obfsct` 文件。
    #[arg(short, long)]
    pub filter: Option<PathBuf>,
    /// 反混淆。
    #[arg(short, long)]
    pub deobfuscate: bool,
    /// 指定工作目录，默认为当前目录。
    pub root: Option<PathBuf>,
}
