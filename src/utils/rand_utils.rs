use weighted_rand::builder::*;
use rand::{Rng, rngs::ThreadRng};

pub fn random_weighted_choice<T: Clone>(choices: &[(T, u32)]) -> T {
    let weights = choices.iter().map(|(_, weight)| *weight).collect::<Vec<u32>>();
    let builder = WalkerTableBuilder::new(&weights);
    let wa_table = builder.build();
    let index = wa_table.next();
    choices[index].0.clone()
}

pub fn boolean_with_probability(probability: f64) -> bool {
    let mut rng = rand::rng();
    let roll: f64 = rng.random();
    roll < probability
}

pub fn gaussian_sample(rng: &mut ThreadRng, mean: f64, std_dev: f64) -> f64 {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();

    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    z0 * std_dev + mean
}

pub fn small_delta(rng: &mut ThreadRng, base: f64) -> f64 {
    let base_scale = (base.abs().max(1.0) * 0.05).max(1.0);
    let raw = gaussian_sample(rng, 0.0, base_scale);
    raw.clamp(-base_scale * 100.0, base_scale * 100.0)
}

pub fn poisson_sample(rng: &mut ThreadRng, lambda: f64) -> u32 {
    let l = (-lambda).exp();
    let mut k = 0;
    let mut p = 1.0;

    loop {
        k += 1;
        let u: f64 = rng.random();
        p *= u;
        if p <= l {
            break;
        }
    }

    (k - 1) as u32
}