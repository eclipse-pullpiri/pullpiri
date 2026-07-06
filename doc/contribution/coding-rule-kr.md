<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri 코딩 규칙

Pullpiri는 Rust 프로그래밍 언어를 사용하여 개발됩니다.  
언어 문법, 의미론, Rust 사용 방법과 관련된 사항은 공개적으로 제공되는 표준 Rust 문서를 참고하고 준수해야 합니다.

## 폴더 이름

폴더 이름은 소문자와 하이픈(`-`)을 사용하여 작성해야 합니다.

- `src`: 소스 코드 폴더
- `tests`: 테스트 코드 폴더
- `examples`: 예제 코드 폴더
- `docs`: 문서 폴더

## 파일 이름

파일 이름은 소문자와 언더스코어(`_`)를 사용하여 작성해야 합니다.

- `main.rs`: 메인 파일
- `lib.rs`: 라이브러리 파일
- `mod_name.rs`: 모듈 파일

## 변수 이름

변수 이름은 소문자와 언더스코어(`_`)를 사용하여 작성해야 합니다.

- `user_name`
- `total_count`
- `is_valid`

## 함수 이름

함수 이름은 소문자와 언더스코어(`_`)를 사용하여 작성해야 합니다.

- `calculate_sum`
- `fetch_data`
- `is_valid_user`

## 코딩 규칙

1. **일관성**: 일관된 코드 스타일을 유지해야 합니다.

2. **명확한 이름 사용**: 변수, 함수, 모듈의 이름은 각각의 역할을 명확히 나타낼 수 있도록 작성해야 합니다.

3. **주석**: 코드는 자체적으로 이해하기 쉽게 작성되어야 하며, 문서는 간결해야 합니다.

   a. 코드의 의도를 설명해야 하는 경우 필요한 위치에 주석을 작성합니다.

   b. 파일 헤더 주석에는 저작권 고지를 포함해야 합니다.

   c. 작업 협업을 돕기 위해 주석에 `FIXME`와 `TODO`를 사용할 수 있습니다.

   d. 일반 주석은 `//` 또는 `/* ... */` 형식을 사용합니다.

   e. 문서화 주석은 `///`, `//!` 또는 `/** ... */` 형식을 사용합니다.

4. **모듈화**: 가독성과 재사용성을 높이기 위해 기능을 모듈화해야 합니다.

5. **오류 처리**: 안정성을 높이기 위해 오류를 철저히 처리해야 합니다.

   a. 복구 가능한 오류에는 `Result<T, E>`를 사용합니다.

   b. 복구할 수 없는 오류에는 `panic!`을 사용합니다.

6. **테스트 작성**: 코드의 신뢰성을 보장하기 위해 테스트 코드를 작성해야 합니다.

7. **표준 라이브러리 사용**: 효율성을 높이기 위해 가능한 한 Rust 표준 라이브러리를 사용해야 합니다.

8. **Cargo.toml 규칙에 따라 라이선스 정보와 의존성 추가**: 빌드 및 정적 검사를 수행하는 데 도움이 되도록 라이선스 정보와 의존성을 `Cargo.toml`에 추가해야 합니다.

   `license` 필드에는 유효한 SPDX 표현식을 사용해야 하며, 유효한 SPDX 라이선스 이름을 포함해야 합니다. 단, 널리 사용되는 관례에 따라 `OR` 대신 `/`를 사용할 수 있습니다. 예를 들어 `MIT/Apache-2.0`과 같이 작성할 수 있습니다.

## 예제 코드

```rust
// src/main.rs
fn main() {
    let user_name = "Alice";
    let total_count = calculate_sum(5, 10);
    println!("Hello, {}! The total count is {}.", user_name, total_count);
}

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

// src/lib.rs
pub fn fetch_data() -> String {
    String::from("Sample data")
}

// src/tests/mod_name.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_sum() {
        assert_eq!(calculate_sum(2, 3), 5);
    }

    #[test]
    fn test_fetch_data() {
        assert_eq!(fetch_data(), "Sample data");
    }
}
