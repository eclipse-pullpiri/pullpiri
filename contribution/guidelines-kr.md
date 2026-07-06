<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# GitHub 개발 워크플로우 규칙 가이드

**작성일**: 2025-04-17
**작성자**: edo-lee_LGESDV

## 목차
1. [이슈 등록 룰](#1-이슈-등록-룰)
2. [브랜치 생성 룰](#2-브랜치-생성-룰)
3. [커밋 룰](#3-커밋-룰)
4. [단계별 라벨 작성 룰](#4-단계별-라벨-작성-룰)
5. [워크플로우 단계별 가이드](#5-워크플로우-단계별-가이드)
6. [자동화 설정 가이드](#6-자동화-설정-가이드)

---

## 1. 이슈 등록 룰

### 이슈 유형 분류
- **FEATURE**: 요구사항 이슈 (부모 이슈)
- **TASK**: 개발 작업 이슈 (하위 이슈)
- **BUG**: 버그 수정 이슈


### 이슈 제목 형식
```
[유형] 제목
```
예시:
- `[FEATURE] 사용자 인증 시스템 구현`
- `[TASK] 로그인 페이지 UI 개발`
- `[BUG] 비밀번호 재설정 이메일 전송 실패`

### 이슈 본문 템플릿

#### 요구사항(REQ) 이슈 템플릿
```markdown
---
name: 요구사항
about: 새로운 기능 요구사항
title: '[FEATURE] '
labels: requirement, status:backlog
assignees: ''
---

## 📝 요구사항 설명
<!-- 요구사항에 대한 상세 설명 -->

## 📋 수용 기준
- [ ] 기준 1
- [ ] 기준 2

## 📎 관련 문서/참조
<!-- 관련 문서 링크 -->

## 📌 하위 작업
<!-- 자동 업데이트됨 -->

## 🧪 테스트 계획
- [ ] 단위 테스트:
- [ ] 통합 테스트:
- [ ] 성능 테스트:

## 📊 테스트 결과
<!-- 이슈 종료 후 자동으로 업데이트됨 -->
```

#### 개발 작업(TASK) 이슈 템플릿
```markdown
---
name: 개발 작업
about: 구현해야 할 개발 작업
title: '[TASK] '
labels: task, status:todo
assignees: ''
---

## 📝 작업 설명
<!-- 수행해야 할 작업 설명 -->

## 📋 체크리스트
- [ ] 항목 1
- [ ] 항목 2

## 🔗 연관 요구사항
<!-- "Relates to #이슈번호" 형식으로 부모 요구사항 연결 -->
Relates to #

## 📐 구현 가이드라인
<!-- 구현 시 참고할 내용 -->

## 🧪 테스트 방법
<!-- 구현 후 테스트 방법 -->
```

### 이슈 관계 설정
- 요구사항(REQ)과 개발 작업(TASK) 연결: TASK 이슈 설명에 `Relates to #요구사항_번호` 명시
- 요구사항 이슈에서 태스크 리스트로 하위 작업 추적:
  ```markdown
  ## 📌 하위 작업
  - [ ] #123 로그인 페이지 UI 개발
  - [ ] #124 백엔드 인증 API 구현
  ```

---

## 2. 브랜치 생성 룰

### 브랜치 명명 규칙
```
<유형>/<이슈번호>-<간략한-설명>
```

### 브랜치 유형
- **feat**: 새로운 기능 개발
- **fix**: 버그 수정
- **refactor**: 코드 리팩토링
- **docs**: 문서 작업
- **test**: 테스트 코드 작업
- **chore**: 기타 유지보수 작업

### 예시
- `feat/123-user-authentication`
- `fix/145-password-reset-bug`
- `docs/167-api-documentation`

### 브랜치 생성 절차
1. 이슈 페이지에서 "Development" > "Create a branch" 이용하거나
2. 명령행에서:
```bash
git checkout -b feat/123-user-login main
```

---

## 3. 커밋 룰

### 커밋 메시지 형식
```
<유형>(<범위>): <설명> [#이슈번호]
```

### 커밋 유형
- **feat**: 새로운 기능
- **fix**: 버그 수정
- **docs**: 문서 변경
- **style**: 코드 포맷팅, 세미콜론 누락 등
- **refactor**: 코드 리팩토링
- **test**: 테스트 관련 코드
- **chore**: 빌드 작업, 패키지 매니저 설정 등

### 예시
- `feat(auth): 소셜 로그인 구현 [#123]`
- `fix(ui): 모바일에서 버튼 오버랩 수정 [#145]`
- `docs(api): API 문서 업데이트 [#167]`

### 커밋 상세 설명 (선택사항)
```
<유형>(<범위>): <설명> [#이슈번호]

<상세 설명>

<주의 사항 또는 Breaking Changes>

<관련 이슈 (Closes, Fixes, Resolves)>
```

### PR 본문 형식
```markdown
## 📝 PR 설명
<!-- 변경 사항에 대한 설명 -->

## 🔗 관련 이슈
<!-- PR이 해결하는 이슈 링크 (Closes, Fixes, Resolves 키워드 사용) -->
Closes #

## 🧪 테스트 방법
<!-- 테스트 방법 설명 -->

## 📸 스크린샷
<!-- UI 변경이 있는 경우 스크린샷 첨부 -->

## ✅ 체크리스트
- [ ] 코드 컨벤션을 준수했습니다
- [ ] 테스트를 추가/수정했습니다
- [ ] 문서를 업데이트했습니다 (필요한 경우)
```

---

## 4. 단계별 라벨 작성 룰

### 라벨 체계

#### 1. 상태 라벨 (status:*)
- `status:backlog` - 백로그에 있는 이슈
- `status:todo` - 할 일 목록에 있는 이슈
- `status:in-progress` - 진행 중인 이슈
- `status:review` - 리뷰 중인 상태
- `status:blocked` - 차단된 상태
- `status:done` - 완료된 이슈

#### 2. 유형 라벨 (type:*)
- `type:requirement` - 요구사항 이슈
- `type:task` - 개발 작업 이슈
- `type:bug` - 버그 이슈
- `type:enhancement` - 기능 개선
- `type:documentation` - 문서화 작업

#### 3. 우선순위 라벨 (priority:*)
- `priority:critical` - 최우선 처리
- `priority:high` - 높은 우선순위
- `priority:medium` - 중간 우선순위
- `priority:low` - 낮은 우선순위

#### 4. 테스트 상태 라벨 (test:*)
- `test:pending` - 테스트 대기 중
- `test:running` - 테스트 실행 중
- `test:passed` - 테스트 통과
- `test:failed` - 테스트 실패

### 라벨 색상 가이드
```
상태 라벨: 파란색 계열
유형 라벨: 녹색 계열
우선순위 라벨: 빨간색/노란색 계열
복잡도 라벨: 보라색 계열
테스트 상태 라벨: 회색/검정색 계열
```

---

## 5. 워크플로우 단계별 가이드

### 1. 요구사항 이슈 등록
- 제목: `[REQ] 요구사항 제목`
- 라벨: `type:requirement`, `status:backlog`
- 상세 내용 작성

### 2. 개발 작업 이슈 등록
- 제목: `[TASK] 작업 제목`
- 라벨: `type:task`, `status:todo`
- 부모 이슈 연결: `Relates to #요구사항번호`

### 3. 브랜치 생성 및 개발
- 브랜치명: `feat/이슈번호-작업명`
- 이슈 상태 변경: `status:in-progress`

### 4. 커밋 및 푸시
- 커밋 메시지: `feat(범위): 구현 내용 [#이슈번호]`

### 5. PR 생성
- 제목: `[이슈유형] 이슈 제목 (#이슈번호)`
- 본문에 `Closes #이슈번호` 포함
- 라벨: `status:review`

### 6. 코드 리뷰 및 머지
- 리뷰어 지정
- 승인 후 머지
- 이슈 자동 종료

### 7. 테스트 실행
- 테스트 실행 트리거
- 테스트 결과에 따라 라벨 업데이트: `test:passed` 또는 `test:failed`
- 요구사항 이슈에 테스트 결과 업데이트

---

## 6. 자동화 설정 가이드

### 브랜치 보호 규칙
1. 저장소 > Settings > Branches > Branch protection rules
2. main/master 브랜치 보호 규칙 설정:
   - Require pull request reviews
   - Require status checks to pass
   - Require linear history

### 라벨 자동화 워크플로우
GitHub Actions으로 다음 자동화 구현:
- 이슈/PR 생성 시 초기 라벨 설정
- 브랜치 생성 시 이슈 상태 업데이트
- PR 머지 시 테스트 실행 및 라벨 업데이트

---

## 워크플로우 다이어그램

```
요구사항 이슈 생성 (adminstrator)
       ↓
  하위 작업 생성 (adminstrator)
       ↓
  브랜치 생성 (adminstrator)
       ↓
   포크 레포 (developer)
       ↓
    개발 작업 (developer)
       ↓
  커밋 및 푸시 (developer)
       ↓
    PR 생성 (developer)
       ↓
  코드 리뷰 (adminstrator)
       ↓
  PR 승인 및 머지 (adminstrator)
       ↓
  자동 테스트 실행 (adminstrator)
       ↓
  이슈 종료 및 결과 업데이트 (adminstrator)
```

---
