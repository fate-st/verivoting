fn main() {
    let winner = vv_types::winner(&[4, 2, 4], 10).expect("well-formed scores");
    println!("four-voter fixture winner: candidate {winner}");
}
