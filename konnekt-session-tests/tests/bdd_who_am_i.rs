use cucumber::World;
use konnekt_session_tests::SessionWorld;

#[path = "steps/who_am_i_steps.rs"]
mod who_am_i_steps;

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
        .run_and_exit("tests/features/who_am_i.feature")
        .await;
}
