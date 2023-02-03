This version of the code only uses the Rolling Hashes to compare Signatures.
This means the probability of a hash collision (and thus for the algorithm to fail)
is considerable.

The files under `tests/random/big/TKzhT` are an example.
The algorithm fails to reconstruct the correct file (`file2`) because
of a hash collision.

The rsync algorithm solves this issue by using both Rolling Hashes and Strong Hashes when
comparing Signatures. The chance of collision is *much* lower that way, and practically impossible.