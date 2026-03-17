use common::persistency;

#[tokio::main]
async fn main() {
    let r = persistency::get("nodes/acrn-NUC11TNHi5").await;
    println!("nodes/acrn-NUC11TNHi5 = {:?}", r);
    
    let r2 = persistency::get("cluster/nodes/acrn-NUC11TNHi5").await;
    println!("cluster/nodes/acrn-NUC11TNHi5 = {:?}", r2);
}
