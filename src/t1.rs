fn main() {
    let files = std::fs::read_dir("/Users/joshuacoles/Library/Mobile Documents/iCloud~com~bigpaua~LearnerCoacher/Documents/Export/JSON/Daily/").unwrap();
    for file in files {
        let file = file.unwrap().path();
        println!("{}\t{}", file.display(), sha256::try_digest(&file).unwrap())
    }
}
