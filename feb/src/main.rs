use num_traits::{identities::Zero, One};
fn main() {

    pub fn fibonacci(n: i64) -> i64 {
        if n.is_zero() {
            panic!("zero is not a right argument to fibonacci()!");
        } else if n.is_one(){
            return 1;
        }
    
        let mut sum = 0;
        let mut last = 0;
        let mut curr = 1;
        for _i in 1..n {
            sum = last + curr.clone();
            last = curr;
            curr = sum.clone();
        }
        sum
    }

    println!("Fibonnacci : {:?}", fibonacci(90));

}
