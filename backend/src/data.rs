use csv::{Reader, StringRecord};
use std::{fs::File, error::Error, path::Path, collections::HashSet};

#[derive(Clone, PartialEq, Debug)]
pub struct Kanji{
    pub kanji: String,
    pub frequency: i32,
}

pub fn vectorize_word_list(path: &str) -> Result<HashSet<String>, Box<dyn Error>> {
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
pub fn vectorize_joyo_kanji<I, P>(paths: I) -> Result<Vec<Vec<Kanji>>, Box<dyn Error>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    // A vector that holds each JLPT level of kanji in its own vector
    let mut kanji_list: Vec<Vec<Kanji>> = Vec::new();

    for path in paths {
        let mut kanji_vec = Vec::new();
        let mut rdr: Reader<_> = Reader::from_path(path)?;

        for result in rdr.records() {
            let record: StringRecord = result?;

            if let Some(kanji_char) = record.get(0) {
                let frequency = record.get(1)
                    .and_then(|f| {
                        if f == "NaN" || f.is_empty() {
                            None
                        } else {
                            f.parse::<i32>().ok()
                        }
                    })
                    .unwrap_or(-1);
                kanji_vec.push(Kanji {
                        kanji: kanji_char.to_owned(),
                        frequency,
                });
            }
        }
        if kanji_vec.is_empty() {
            return Err("Empty kanji file".into());
        }
        kanji_list.push(kanji_vec);
    }

    Ok(kanji_list)
}
