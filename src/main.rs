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

fn is_valid_word(guess: &str, word_list: Result<Vec<String>, Box<dyn Error>>) -> bool {
    match word_list {
        // user .iter().any() to compare &String and &str directly
        Ok(words) => words.iter().any(|word| word == guess),
        Err(_) => false,
    }
}

fn is_valid_kanji(guess: &str, kanji: &str) -> bool {
    guess.contains(kanji)
}

fn main() {
    let word_list = vectorize_word_list("./data/kanji_words.csv");
    let kanji_list = vectorize_joyo_kanji("./data/joyo_kanji.csv");
    match kanji_list {
        Ok(kanji_list) => {
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

                // as of now no need to handle Result because it is handled in is_valid_word.
                // not sure if that is good...
                let good_on_kanji: bool = is_valid_kanji(guess, random_kanji);
                let good_on_word: bool = is_valid_word(guess, word_list);

                if good_on_kanji && good_on_word {
                    println!("Good Guess!");
                } else if good_on_kanji && !good_on_word {
                    println!("Bad Guess: Correct kanji, but not a valid word...");
                } else if !good_on_kanji && good_on_word {
                    println!("Bad Guess: Valid word, but does not contain the correct kanji you were supposed to use...");
                } else {
                    print!("Bad guess: Incorrect kanji and not a valid word");
                }
            }
        }
        Err(err) => {
            print!("Error: {}", err);
        }
    }
}
