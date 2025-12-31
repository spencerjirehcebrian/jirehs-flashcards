use jirehs_flashcards_backend::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
