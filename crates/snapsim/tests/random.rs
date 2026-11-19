use snapsim::random::PythonRandom;

#[test]
fn matches_cpython_fixtures() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../parity/rng_fixtures.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let seed: i128 = case["seed"].as_str().unwrap().parse().unwrap();
        let mut rng = PythonRandom::seed(seed);
        for pair in case["bits"].as_array().unwrap() {
            let actual = rng.getrandbits(pair[0].as_u64().unwrap() as u32);
            let expected: u128 = pair[1].as_str().unwrap().parse().unwrap();
            assert_eq!(actual, expected, "seed {seed}, bits {}", pair[0]);
        }
        for value in case["random"].as_array().unwrap() {
            assert_eq!(rng.random(), value.as_f64().unwrap(), "seed {seed}");
        }
        for pair in case["randbelow"].as_array().unwrap() {
            assert_eq!(
                rng.randbelow(pair[0].as_u64().unwrap() as usize),
                pair[1].as_u64().unwrap() as usize,
                "seed {seed}"
            );
        }
        for range in case["range"].as_array().unwrap() {
            assert_eq!(
                rng.randrange(
                    range[0].as_i64().unwrap(),
                    range[1].as_i64().unwrap(),
                    range[2].as_i64().unwrap()
                ),
                range[3].as_i64().unwrap(),
                "seed {seed}"
            );
        }
        for value in case["randint"].as_array().unwrap() {
            assert_eq!(
                rng.randint(-100, 100),
                value.as_i64().unwrap(),
                "seed {seed}"
            );
        }
        for (n, value) in [1, 2, 3, 4, 17, 128]
            .into_iter()
            .zip(case["choice"].as_array().unwrap())
        {
            let values: Vec<_> = (0..n).collect();
            assert_eq!(*rng.choice(&values), value.as_i64().unwrap(), "seed {seed}");
        }
        for expected in case["shuffle"].as_array().unwrap() {
            let expected: Vec<_> = expected
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect();
            let mut values: Vec<_> = (0..expected.len()).collect();
            rng.shuffle(&mut values);
            assert_eq!(values, expected, "seed {seed}");
        }
        for sample in case["sample"].as_array().unwrap() {
            let expected: Vec<_> = sample[2]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect();
            assert_eq!(
                rng.sample_indices(
                    sample[0].as_u64().unwrap() as usize,
                    sample[1].as_u64().unwrap() as usize
                ),
                expected,
                "seed {seed}"
            );
        }
        let state = rng.getstate();
        let expected = case["state"].as_array().unwrap();
        assert_eq!(
            state.words.as_slice(),
            expected[..624]
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>()
        );
        assert_eq!(state.index, expected[624].as_u64().unwrap() as usize);
        for _ in 0..2 {
            rng.setstate(state.clone());
            for word in case["after_state"].as_array().unwrap() {
                assert_eq!(rng.getrandbits(32), word.as_u64().unwrap() as u128);
            }
        }
    }
}
