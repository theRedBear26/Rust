mod mini_bitcask;

use mini_bitcask::MiniBitcask;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = MiniBitcask::new(PathBuf::from("data.db"))?;

    // Store data
    db.set(b"name", b"Alice".to_vec())?;

    // Read data
    let value = db.get(b"name")?;

    println!("Value: {:?}", value);

    // Delete data
    db.delete(b"name")?;

    // Merge database
    db.merge()?;

    Ok(())
}