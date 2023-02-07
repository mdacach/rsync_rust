# rsync_rust

A simplified rsync algorithm from [Andrew Tridgell's Ph.D. thesis](https://www.samba.org/~tridge/phd_thesis.pdf).
This project is meant for learning purposes *only*. It does not aim to fulfill the characteristics of rsync.
This code uses my implementation of a [`rolling_hash`](https://github.com/mdacach/rolling_hash_rust).

## Main idea

User A has an initial file. Let's call it `basis_file`.  
User B has made some changes to this file and has its own version of it. Let's call it `updated_file`.  
Now, User B wants to propagate its changes to User A, so they can both have the more recent file.

One way of accomplishing that is for User B to send their file directly to User A:

1. User B sends `updated_file` to User A
2. User A replaces `basis_file` with `updated_file`

This, of course, works, but we are not leveraging the fact that both `basis_file` and `updated_file`
are very similar (one is an updated version of the other).

(Suppose `basis_file` is some file in a Git repository,
and `updated_file` is the same file after you've made some commits).

## The rsync Algorithm

The rsync algorithm aims to improve the network usage of such interaction by lowering the amount of resources we need to
send between the connections. While the first approach would send the whole `updated_file` throughout the network
(which can be quite big),
rsync will try to improve on this result by exploiting the similarities between the two files:

1. User A computes a `signature` for `basis_file`.  
   <sub> This `signature` "represents" the contents of `basis_file` approximately, and is much smaller. </sub>
2. User A sends the `signature` to User B.  
   <sub> As `signature` is much smaller than `basis_file`, this is not resource-intensive. </sub>
3. User B uses `signature` to compute a `delta` from `basis_file` to `updated_file`.  
   <sub> This `delta` has what is needed to change from `basis_file` to `updated_file`. In case the two files are
   similar, the delta is small. </sub>
4. User B sends `delta` to User A.
5. User A uses `delta` to update `basis_file`.

During the process, we have sent two files throughout the network: `signature` and `delta`.
As long as `size(signature)` + `size(delta)` is smaller than `size(updated_file)`, we have made
improvements regarding network resources.

**Note** If the networking capabilities between the two users is asymmetric,
this may not be an improvement.

**Note** We have traded computation time for memory.
While this algorithm sends fewer resources through the network, it requires both User A and User B to process the files.
(User A computes a signature and updates a file, and User B computes a delta).

## Glossary

These terms are used throughout the code.

- `basis_file` - file that user A has.
- `updated_file` - file that user B has, generally an updated version of `basis_file`.
- `signature` - file representing approximately the contents of `basis_file`.
- `delta` - file representing the differences between `basis_file` and `updated_file`.
- `recreated` - resulting file after applying the `delta` to `basis_file`.

## The Code

The code has three main functions, which are used through the command line.
In pseudocode:

1. `compute_signature(basis_file) -> signature`
2. `computed_delta(signature, updated_file) -> delta`
3. `apply_delta(basis_file, delta) -> recreated`

## Testing Methodology

A `TestCase` consists of a `basis_file` and a `updated_file`.
The test succeeds if we are able to recreate the `update_file` exactly by running the three main functions in order.  
The repository has `TestCases` examples under `/tests/integration_tests/test_files/`.
The integration tests are #[ignored] by default as I could not get them running correctly in the CI.

```cargo test --test integration_tester run_all_test_files -- --ignored --nocapture```  
will run all the test_files in the folder.

3 - Compression tests - using Linux source code and explained below.

## Linux source code analysis

I ran the algorithm against two versions of the Linux Source Code,
stable [v6.1.9](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tag/?h=v6.1.9)
and [v6.1.8](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tag/?h=v6.1.8) releases.
The Linux source code is a good dataset because:

- The dataset is considerably big (each release is around 1.3Gb), but the differences between versions are small.
- It is a real use-case for rsync algorithm

The efficiency of the algorithm depends primarily on:

- The degree of similarity between the files
- The block size used for dividing each file

![Image with measurements from Linux source. The image shows block sizes of around 3000 bytes to
be the better performing, with worse values for too-small blocks (< 100) and too-big blocks (> 10000)](./analysis/linux_plot.png)

Blocks of around 3000 bytes were the best performing, with up to 18 compression ratio.  
Following our intuition, both too-small blocks and too-big blocks do not perform well.

If a block is too small, `signature` alone will be too big.  
If a block is too big, there's a very small chance of matching it with our file.

The measurements for this plot can be found [here](./analysis/linux_chunk_size_to_compression_ratio.csv).

**Note**
As the code does not support multiple files yet, the hacky way I've got this to work was to
merge every source code file together into a very big one. Then those two files (one for each version release)
were tested through the algorithm. This is obviously not ideal and will be corrected later, but for now, it works.

## References:

* [Andrew Tridgell's Ph.D. thesis](https://www.samba.org/~tridge/phd_thesis.pdf)
* [rsync tech report](https://rsync.samba.org/tech_report/tech_report.html)
* [rdiff manual page](https://man.archlinux.org/man/rdiff.1.en)
* [MIT OCW lecture](https://www.youtube.com/watch?v=BRO7mVIFt08)
* [MIT OCW recitation](https://www.youtube.com/watch?v=w6nuXg0BISo)
* [Algorithms Live](https://www.youtube.com/watch?v=rA1ZevamGDc)
* [cp-algorithms](https://cp-algorithms.com/string/string-hashing.html#calculation-of-the-hash-of-a-string)
* [my rolling hash implementation](https://github.com/mdacach/rolling_hash_rust)

## TODO

1. Add example usage to README
2. Improve error handling throughout the project
3. Add multiple file processing functionality, as described in the paper.
4. Improve the code as I get more familiar with Rust.
