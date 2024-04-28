pub type Matrix<const N: usize> = [[u64; N]; N];

fn init_matrix<const N: usize>() -> Matrix<N> {
    let mut matrix = [[0u64; N]; N];
    for i in 0..N {
        for j in 0..N {
            matrix[i][j] = (i * N + j + 1) as u64;
        }
    }
    matrix
}

fn matrix_multiply<const N: usize>(matrix1: &Matrix<N>, matrix2: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0u64; N]; N];

    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                result[i][j] += matrix1[i][k] * matrix2[k][j];
            }
        }
    }

    result
}

pub fn matrix_test<const N: usize>() {
    let a = init_matrix::<N>();
    let b = init_matrix::<N>();
    let _c = matrix_multiply::<N>(&a, &b);
}