// Module `mighty.misnamed`, which declares another name. A module file opens
// with the canonical name its position gives, and loading it holds the two to
// each other; this file is loaded only to be refused.

module mighty.renamed
fn hello() : String = "hello"
