fn main() {
    if let Err(error) = noisemaker_cpu::cli::run() {
        eprintln!("noisemaker-rs: {error}");
        std::process::exit(1);
    }
}
