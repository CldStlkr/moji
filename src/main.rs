use core::panic;
use csv::{Reader, StringRecord};
use rand::Rng;
use std::{
    error::Error,
    fs::File,
    io::{self, Write},
};

fn vectorize_word_list(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut word_list: Vec<String> = Vec::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        let word: &str = record.get(0).unwrap_or("N/A");

        word_list.push(String::from(word));
    }

    Ok(word_list)
}

fn vectorize_joyo_kanji(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut kanji_list: Vec<String> = Vec::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        let kanji: &str = record.get(0).unwrap_or("N/A");

        kanji_list.push(String::from(kanji));
    }

    Ok(kanji_list)
}

fn is_valid_word(guess: &str, word_list: &Vec<String>) -> bool {
    for word in word_list.iter() {
        if guess == word {
            return true;
        }
    }

    false
}

fn is_valid_kanji(guess: &str, kanji: &str) -> bool {
    guess.contains(kanji)
}

fn main() {
    let word_list: Vec<String> =
        vectorize_word_list("./data/kanji_words.csv").expect("Failed to vectorize word list");

    let kanji_list: Vec<String> =
        vectorize_joyo_kanji("./data/joyo_kanji.csv").expect("Failed to vectorize kanji list");

    let mut looping: bool = true;

    while looping {
        let mut rand = rand::thread_rng();
        let random_index = rand.gen_range(0..kanji_list.len());
        if let Some(random_kanji) = kanji_list.get(random_index) {
            println!("Provide a word that contains this {} kanji.", random_kanji);
            print!("\nType your word here: ");

            io::stdout().flush().expect("failed to flush stdout");

            let mut guess = String::new();
            io::stdin()
                .read_line(&mut guess)
                .expect("failed to read line");
            let guess = guess.trim();

            let good_kanji: bool = is_valid_kanji(guess, random_kanji);
            let good_word: bool = is_valid_word(guess, &word_list);

            if good_kanji && good_word {
                println!("Good Guess!");
            } else if good_kanji && !good_word {
                println!("Bad Guess: Correct kanji, but not a valid word...");
                looping = false;
            } else if !good_kanji && good_word {
                println!("Bad Guess: Valid word, but does not contain the correct kanji you were supposed to use...");
                looping = false;
            } else {
                println!("Bad guess: Incorrect kanji and not a valid word.");

                looping = false;
            }
            if !looping {
                let mut correct_words: Vec<&str> = Vec::new();
                for word in word_list.iter() {
                    if word.contains(random_kanji) {
                        correct_words.push(word);
                    }
                }
                if !correct_words.is_empty() {
                    println!("Here are some correct words: {:?}", correct_words);
                } else {
                    println!("No correct words found.");
                }
            }
        } else {
            panic!("Could not pull random kanji from kanji list");
        }
    }
}
