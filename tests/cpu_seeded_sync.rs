use noisemaker_cpu::{CpuRenderer, RenderOptions};
use sha2::{Digest, Sha256};

fn check_case(name: &str) {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/cpu-sync/seeded-cases.json")).unwrap();
    assert_eq!(
        fixtures["oracleRevision"],
        "4aeb3e1e028b2dc412401f7b7f17c014c603bf14"
    );
    let cases = fixtures["cases"].as_array().unwrap();
    let case = cases
        .iter()
        .find(|case| case["name"].as_str() == Some(name))
        .expect("CPU capture exists");
    {
        let options = RenderOptions {
            width: case["width"].as_u64().unwrap() as u32,
            height: case["height"].as_u64().unwrap() as u32,
            time: case["time"].as_f64().unwrap() as f32,
            seed: case["seed"].as_i64().unwrap() as i32,
            ..RenderOptions::default()
        };
        let pixels = CpuRenderer::new()
            .unwrap()
            .render(case["program"].as_str().unwrap(), &options)
            .unwrap()
            .surface
            .to_rgba8();
        assert_eq!(
            format!("{:x}", Sha256::digest(pixels)),
            case["rgbaSha256"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}

macro_rules! cpu_case {
    ($function:ident, $name:literal) => {
        #[test]
        fn $function() {
            check_case($name);
        }
    };
}
cpu_case!(cpu_sync_perlin_3d, "perlin3d-41x23-t0.25-s13");
cpu_case!(cpu_sync_noise_3d, "synth3d__noise3d__s7");
cpu_case!(cpu_sync_cell_3d, "synth3d__cell3d__s7");
cpu_case!(cpu_sync_reaction_seeding, "reaction-seeding");
cpu_case!(cpu_sync_cellular_seeding, "cellular-seeding");
cpu_case!(cpu_sync_scatter, "scatter");
cpu_case!(cpu_sync_extrude, "extrude");
cpu_case!(cpu_sync_spooky_ticker, "spookyTicker");
