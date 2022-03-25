use rand::Rng;

const ARTIFACT_SYLLABLES: [&str; 11] = [
    "gi", "reh", "han", "do", "mee", "sak", "ein", "pol", "maat", "hen", "kid"
];

pub fn generate_artifact_name(min_syllables: i32, max_syllables: i32) -> String {
    // let artifact_syllables: Vec<&str>=  vec![
    //     "gi", "reh", "han", "do", "mee", "sak", "ein", "pol", "maat", "hen", "kid"
    // ];
    
    let num_syllables = rand::thread_rng().gen_range(min_syllables, max_syllables);
    let mut name = String::from("");

    for _ in 0..num_syllables {
        let selection = ARTIFACT_SYLLABLES[rand::thread_rng().gen_range(0, ARTIFACT_SYLLABLES.len())];
        name = name + selection.into()
    }
    return name
}