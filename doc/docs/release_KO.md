<!--
SPDX-FileCopyrightText: Copyright 2024-25 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri 릴리스 관리

- [개요](#개요)
- [릴리스 프로세스](#릴리스-프로세스)
  - [버전 관리](#버전-관리)
  - [릴리스 생성](#릴리스-생성)
  - [자동 릴리스 파이프라인](#자동-릴리스-파이프라인)
- [릴리스 아티팩트](#릴리스-아티팩트)
- [릴리스 검증](#릴리스-검증)

## 개요

Pullpiri는 GitHub Actions를 사용하여 자동화된 CI/CD 파이프라인으로 릴리스 프로세스를 관리합니다. 버전 태그가 저장소에 푸시되면 워크플로우가 자동으로:

1. 코드베이스 검증 (포매팅, 린팅, 테스트)
2. 문서 및 설정 검증
3. 코드 커버리지 및 규정 준수 리포트 생성
4. 릴리스 아티팩트 및 바이너리 컴파일
5. GitHub Release에 자산 발행

이러한 자동화된 접근 방식은 각 단계에서 종합적인 검증을 통해 일관되고 고품질의 릴리스를 보장합니다.

## 릴리스 프로세스

### 버전 관리

Pullpiri는 **vX.Y.Z** 형식의 의미있는 버전 관리(Semantic Versioning)를 따릅니다.

- **X**: 주(Major) 버전 (호환되지 않는 변경사항)
- **Y**: 부(Minor) 버전 (새로운 기능, 하위 호환성 유지)
- **Z**: 패치(Patch) 버전 (버그 수정)

예시: `v1.0.0`, `v1.1.0`, `v1.2.3`

### 릴리스 생성

새로운 릴리스를 생성하려면:

```bash
# 프로젝트 디렉토리로 이동
cd /path/to/pullpiri

# 버전 태그 생성 (예: v1.0.0)
git tag v1.0.0

# 태그를 푸시하여 릴리스 워크플로우 트리거
git push origin v1.0.0
```

**중요**: 태그는 `v`로 시작해야 하며 의미있는 버전 형식을 따라야 합니다. 유효하지 않은 태그는 릴리스 워크플로우를 트리거하지 않습니다.

태그가 푸시되면 GitHub Actions가 자동으로 릴리스 파이프라인을 시작합니다. GitHub 저장소의 **Actions** 탭에서 진행 상황을 모니터링할 수 있습니다.

### 자동 릴리스 파이프라인

릴리스 파이프라인은 **5개의 순차 단계**로 구성되며, 모두 성공해야 발행됩니다:

#### 1단계: Rust 코드베이스 검증 (`run-rust-ci`)

모든 Rust 코드를 프로젝트 표준에 맞게 검증합니다:

- **포매팅 검사** (`cargo fmt --check`): Rust 포매팅 규칙 준수 여부 검증
- **린팅 검사** (`cargo clippy`): 코드 품질 문제 및 스타일 위반 식별
- **단위 테스트** (`cargo test`): 모든 컴포넌트에서 단위 테스트 실행
  - server (apiserver, settingsservice 등)
  - player (filtergateway, actioncontroller, statemanager)
  - tools (pirictl, rocksdb-inspector, idl2rs)
  - common (공유 유틸리티 및 gRPC 정의)

**출력**: fmt_summary.md, clippy_summary.md, test_summary.xml

#### 2단계: 문서 검증 (`run-doc-lint`)

마크다운 파일 및 문서 포매팅 검증:

- 마크다운 구문 검사
- 링크 검증
- 포매팅 일관성

**실패 영향**: 문서 표준을 충족하지 않으면 릴리스 방지.

#### 3단계: YAML 설정 검증 (`run-yaml-validation`)

저장소의 모든 YAML 파일 검증:

- GitHub Actions 워크플로우 (`.github/workflows/*.yml`)
- 설정 파일
- 예제 시나리오

**실패 영향**: YAML이 잘못된 형식이면 릴리스 방지.

#### 4단계: 라이선스 및 의존성 검사 (`run-license-report`)

오픈소스 라이선스 규정 준수 리포트 생성:

- `cargo-about`를 사용한 모든 Rust 의존성 스캔
- 허용 목록(`.cargo/deny.toml`)에 대한 검증
- 라이선스 위반 또는 제한된 의존성 감지
- HTML 라이선스 리포트 생성

**출력**: license-report.html, deny_summary.md

**실패 영향**: 금지된 의존성이 감지되면 릴리스 방지.

#### 5단계: 아티팩트 수집 및 발행 (`tag_release_artifacts`)

이전 단계의 모든 아티팩트를 수집하여 GitHub Release에 발행:

- 이전 단계에서 생성된 모든 리포트 다운로드
- nodeagent 바이너리 컴파일 (AMD64, ARM64)
- 문서 아카이브 생성
- 모든 파일을 GitHub Release 자산으로 업로드
- 자동 릴리스 노트 생성 트리거

---

## 릴리스 아티팩트

릴리스 파이프라인에서 생성된 모든 아티팩트는 GitHub Release 페이지에 자동으로 업로드됩니다.

### 코드 커버리지 리포트

각 컴포넌트별로 생성됩니다:

- `code-coverage-server.html` - API 서버 커버리지
- `code-coverage-tools.html` - 도구(pirictl, rocksdb-inspector) 커버리지
- `code-coverage-common.html` - 공유 유틸리티 커버리지

### 검증 리포트

- `fmt_summary.md` - 포매팅 검사 결과
- `clippy_summary.md` - 린팅/코드 품질 검사 결과
- `test_summary.xml` - 단위 테스트 결과 (JUnit 형식)
- `deny_summary.md` - 의존성 및 라이선스 검사 결과

### 라이선스 리포트

- `license-report.html` - 완전한 오픈소스 라이선스 문서

### 컴파일된 바이너리

- `nodeagent-linux-amd64` - Linux x86_64용 NodeAgent 바이너리
- `nodeagent-linux-arm64` - Linux ARM64용 NodeAgent 바이너리

### 문서 및 스크립트

- `README.md` - 주요 프로젝트 문서
- `coding-rule.md` - 코딩 가이드라인 및 표준
- `release.yml` - 릴리스 워크플로우 정의
- `install_nodeagent.sh` - NodeAgent 설치 스크립트
- `node_ready_check.sh` - 노드 준비 상태 검증 스크립트
- `doc-archive.tar.gz` - 완전한 문서 폴더 아카이브

---

## 릴리스 검증

릴리스 파이프라인은 각 단계에서 종합적인 검증을 구현합니다:

### 릴리스 전 검사

릴리스가 발행되기 전에 다음이 모두 통과해야 합니다:

1. **모든 Rust 코드**가 포매팅, 린팅 및 테스트를 통과해야 함
2. **모든 문서**가 유효한 마크다운 형식을 가져야 함
3. **모든 설정 파일**이 유효한 YAML 구문을 가져야 함
4. **모든 의존성**이 승인되고 적절히 라이선스되어야 함
5. **코드 커버리지**가 품질 평가를 위해 생성되어야 함

### 실패 처리

단계가 실패하면:

1. 워크플로우가 즉시 중지되고 이후 단계가 건너뛰어집니다
2. 릴리스는 GitHub에 **발행되지 않습니다**
3. 자세한 오류 로그는 **Actions** 탭에서 확인 가능합니다
4. 개발자는 문제를 수정하고 새 태그를 푸시하여 다시 시도해야 합니다

### 릴리스 상태 모니터링

릴리스 파이프라인 진행 상황 확인:

1. GitHub 저장소로 이동
2. **Actions** 탭 클릭
3. 버전 태그로 트리거된 워크플로우 찾기
4. 각 단계의 상태 모니터링
5. 실패 시 자세한 로그 검토

---

## 릴리스 노트

GitHub는 다음을 기반으로 자동으로 릴리스 노트를 생성합니다:

- 마지막 릴리스 이후 병합된 커밋
- Pull Request 제목 및 설명
- 기여자 크레딧

릴리스 노트는 GitHub Release 페이지에서 릴리스가 발행된 후 수동으로 편집할 수 있습니다.

<!-- markdownlint-disable-file MD033 no-inline-html -->
