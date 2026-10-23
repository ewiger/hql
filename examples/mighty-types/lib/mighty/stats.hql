// Module `mighty.stats`: descriptive statistics written in HQL over `math`.
import math

fn mean(xs: List<Float>) : Option<Float> =
    none given xs.size() == 0 otherwise some(sum(xs) / float(xs.size()))

fn variance(xs: List<Float>) : Option<Float> =
    mean(xs).map(m => sum(xs.map(x => (x - m) * (x - m))) / float(xs.size()))

fn stddev(xs: List<Float>) : Option<Float> = variance(xs).map(v => math.sqrt(v))

// The middle of a sorted sequence: the one middle value, or the mean of two.
fn middle(sorted: List<Float>) : Option<Float> =
    sorted[sorted.size() / 2] given sorted.size() % 2 == 1
    otherwise sorted[sorted.size() / 2 - 1].and_then(a => sorted[sorted.size() / 2].map(b => (a + b) / 2.0f))

fn median(xs: List<Float>) : Option<Float> = middle(xs | sort)
