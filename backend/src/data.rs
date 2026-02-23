use crate::error::DataLoadError;
use csv::{Reader, StringRecord};
use std::{fs::{read_to_string, File}, path::Path, collections::{HashMap, HashSet}};


pub type KanjiData = Vec<Vec<Kanji>>;
pub type DictData = HashSet<String>;
pub type JlptWordData = Vec<HashMap<String, Vec<String>>>;

#[derive(Clone, PartialEq, Debug)]
pub struct Kanji{
    pub kanji: String,
    pub frequency: i32,
}

pub fn load_dictionary(path: &str) -> Result<DictData, DataLoadError>{
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut word_set = HashSet::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        if let Some(word) = record.get(0) {
            word_set.insert(word.to_string());
        }
    }

    Ok(word_set)
}

/// Loads kanji from multiple CSV files, keeping each file's kanji separate.
/// Returns a Vec where each inner Vec corresponds to one JLPT level.
/// The order of inner Vecs matches the order of input paths.
pub fn vectorize_joyo_kanji<I, P>(paths: I) -> Result<KanjiData, DataLoadError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    // A vector that holds each JLPT level of kanji in its own vector
    let mut kanji_list: Vec<Vec<Kanji>> = Vec::new();

    for path in paths {
        let path = path.as_ref();
        let mut kanji_vec = Vec::new();
        let mut rdr: Reader<_> = Reader::from_path(path)?;

        for result in rdr.records() {
            let record: StringRecord = result?;

            if let Some(kanji_char) = record.get(0) {
                let frequency = record.get(1)
                    .and_then(|f| {
                        if f == "NaN" || f.is_empty() { None }
                        else { f.parse::<i32>().ok() }
                    })
                    .unwrap_or(-1);
                kanji_vec.push(Kanji {
                        kanji: kanji_char.to_owned(),
                        frequency,
                });
            }
        }
        if kanji_vec.is_empty() {
            return Err(DataLoadError::EmptyFile(path.to_path_buf()));
        }
        kanji_list.push(kanji_vec);
    }

    Ok(kanji_list)
}

pub fn load_jlpt_words<I, P>(paths: I) -> Result<JlptWordData, DataLoadError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut word_levels: Vec<HashMap<String, Vec<String>>> = Vec::new();
    for path in paths {
        let path = path.as_ref();
        let mut word_map: HashMap<String, Vec<String>> = HashMap::new();
        let content = read_to_string(path)?;
        for line in content.lines() {
            let mut parts = line.split(',');
            if let Some(word) = parts.next() {
                if word.is_empty() { continue; }
                let readings: Vec<String> = parts.map(|s| s.to_string()).collect();
                if !readings.is_empty() { word_map.insert(word.to_string(), readings); }
            }
        }
        if word_map.is_empty() { return Err(DataLoadError::EmptyFile(path.to_path_buf())); }
        word_levels.push(word_map);
    }

    Ok(word_levels)
}
