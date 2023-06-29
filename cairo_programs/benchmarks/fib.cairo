func main() {
    let res = fib(90);

    assert res = 2880067194370816120;


    ret;
}

func fib(n) -> felt {
    if (n == 0) {
        return 1; 
    }
    
    if (n == 1) {
        return (1);
    }

        let sum = 0;
        let last = 0;
        let curr = 1;

        let i = 0;

        return loop(1,n, sum, last, curr);

    }

func loop(i: felt, n: felt, sum:felt, last: felt, curr: felt) -> felt {
    if (i == n) {
        return sum;
    }

    let sum = last + curr;
    let last = curr;
    let curr = sum;

    return loop(i+1, n, sum, last, curr);
}
