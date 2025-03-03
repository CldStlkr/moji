use csv::{Reader, StringRecord};
use std::error::Error;
use std::fs::File;

pub fn vectorize_word_list(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut word_list = Vec::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        let word: &str = record.get(0).unwrap_or("N/A");
        word_list.push(word.to_string());
    }

    Ok(word_list)
}

pub fn vectorize_joyo_kanji(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut kanji_list = Vec::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        let kanji: &str = record.get(0).unwrap_or("N/A");
        kanji_list.push(kanji.to_string());
    }

    Ok(kanji_list)
}
