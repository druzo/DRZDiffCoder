// Arrow functions + spread + array pipelines.

const numbers = [12, 7, 19, 3, 21, 8];

const isPrime = (n) => {
  if (n < 2) return false;
  for (let d = 2; d * d <= n; d++) {
    if (n % d === 0) return false;
  }
  return true;
};

const primes = numbers.filter(isPrime);
const doubled = [...primes, ...primes.map((p) => p * 2)];

const grouped = doubled.reduce((acc, n) => {
  const key = n % 2 === 0 ? "even" : "odd";
  (acc[key] ||= []).push(n);
  return acc;
}, {});

console.log("primes:", primes);
console.log("grouped:", grouped);