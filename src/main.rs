use bedlam::Node;

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();
    let stdout = std::io::stdout().lock();

    Node::new(stdin, stdout).initialize()?.process_messages()
}
