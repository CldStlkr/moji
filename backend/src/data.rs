use csv::{Reader, StringRecord};
use std::fs::File;
use std::{error::Error, path::Path};

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

pub fn vectorize_joyo_kanji<I, P>(paths: I) -> Result<Vec<String>, Box<dyn Error>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut kanji_list = Vec::new();

    for path in paths {
        let mut rdr: Reader<_> = Reader::from_path(path)?;
        for result in rdr.records() {
            let record: StringRecord = result?;
            if let Some(kanji) = record.get(0) {
                kanji_list.push(kanji.to_owned());
            }
        }
    }

    Ok(kanji_list)
}
