use cucumber::World;
use konnekt_session_tests::SessionWorld;

#[tokio::main]
async fn main() {
    #[cfg(feature = "output-junit")]
    {
        let junit_file =
            std::fs::File::create("junit-report.xml").expect("Failed to create JUnit XML file");

        SessionWorld::cucumber()
            .max_concurrent_scenarios(1)
            .with_writer(cucumber::writer::JUnit::new(junit_file, 0))
            .run("tests/features")
            .await;
        return;
    }

    #[cfg(all(feature = "output-json", not(feature = "output-junit")))]
    {
        let json_file = std::fs::File::create("cucumber-report.json")
            .expect("Failed to create JSON output file");

        SessionWorld::cucumber()
            .max_concurrent_scenarios(1)
            .with_writer(cucumber::writer::Json::new(json_file))
            .run("tests/features")
            .await;
        return;
    }

    #[cfg(not(any(feature = "output-json", feature = "output-junit")))]
    {
        use cucumber::WriterExt;
        SessionWorld::cucumber()
            .max_concurrent_scenarios(1)
            .with_writer(
                cucumber::writer::Basic::raw(
                    std::io::stdout(),
                    cucumber::writer::Coloring::Auto,
                    0,
                )
                .summarized()
                .assert_normalized(),
            )
            .run_and_exit("tests/features")
            .await;
    }
}
