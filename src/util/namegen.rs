use rand::Rng;
use crate::util::string_utils::capitalize;

const ARTIFACT_SYLLABLES: [&str; 11] = [
    "gi", "reh", "han", "do", "mee", "sak", "ein", "pol", "maat", "hen", "kid"
];

const OGUR_SYLLABLES: [&str; 14] = [
    "bo", "kud", "da", "ke", "ku", "sak", "sad", "se", "be", "je", "ju", "juk", "jad", "jak"
];
const OGUR_MIN_SYLLABLES: i32 = 2;
const OGUR_MAX_SYLLABLES: i32 = 5;

pub fn generate_artefact_name() -> String {
    generate_name(&ARTIFACT_SYLLABLES, 2, 7)
}

pub fn generate_ogur_name() -> String {
    generate_name(&OGUR_SYLLABLES, OGUR_MIN_SYLLABLES, OGUR_MAX_SYLLABLES)
}

pub fn generate_name(syllables: &[&str], min_syllables: i32, max_syllables: i32) -> String {
    // let artifact_syllables: Vec<&str>=  vec![
    //     "gi", "reh", "han", "do", "mee", "sak", "ein", "pol", "maat", "hen", "kid"
    // ];

    let num_syllables = rand::thread_rng().gen_range(min_syllables, max_syllables);
    let mut name = String::from("");

    for _ in 0..num_syllables {
        let selection = syllables[rand::thread_rng().gen_range(0, ARTIFACT_SYLLABLES.len())];
        name = name + selection.into()
    }
    return capitalize(name);
}
