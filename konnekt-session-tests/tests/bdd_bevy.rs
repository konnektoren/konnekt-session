use cucumber::World;
use konnekt_session_tests::SessionWorld;

#[path = "steps/bevy_application_steps.rs"]
mod bevy_application_steps;

#[tokio::main]
async fn main() {
    use cucumber::WriterExt;
    SessionWorld::cucumber()
        .max_concurrent_scenarios(1)
        .with_writer(
            cucumber::writer::Basic::raw(std::io::stdout(), cucumber::writer::Coloring::Auto, 0)
                .summarized()
                .assert_normalized(),
        )
        .run_and_exit("tests/features/bevy_application.feature")
        .await;
}
