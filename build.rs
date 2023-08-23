
fn main()
{
    //compilo i programmi (la compilazione deve essere fatta sulla macchina target per mantenere il codice cross platform)
    //per:
    // 1. rilevare la dimensione del display
    std::process::Command::new("cargo")
                            .arg("build")
                            .current_dir(".\\display_resolution_detection")
                            .output()
                            .expect("Failed to compile display_resolution_detection");

    // 2. selezionare il rettangolo di cui fare lo screenshot
    //compilo il programma (la compilazione deve essere fatta sulla macchina target per mantenere il codice cross platform)
    std::process::Command::new("cargo")
                            .arg("build")
                            .current_dir(".\\rect_selection")
                            .output()
                            .expect("Failed to compile rect_selection");
}