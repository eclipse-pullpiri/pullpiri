# Pullpiri Dashboard

Pullpiri Dashboard는 Podman 등 컨테이너 작업을 웹에서 직관적으로 관리할 수 있도록 만든 React 기반 대시보드입니다.  
이 디렉토리의 코드는 Vite 기반 프론트엔드이며, 오픈소스 pullpiri 프로젝트의 일부로 배포됩니다.

## 시작하기

### 1. 의존성 설치

**최상위 pullpiri 프로젝트 루트**에서(즉, `package.json`이 있는 곳)  
(처음 설치라면)
```sh
npm install
```

### 2. 대시보드 개발 서버 실행

**dashboard 폴더로 이동**
```sh
cd src/tools/dashboard
```

**개발 서버 시작**
```sh
npm install      # (최초 1회, 혹은 패키지 추가/업데이트 시)
npm run dev
```

**브라우저에서 접속**
```
http://localhost:5173
```

---

## 폴더 구조

```
src/tools/dashboard/
├── src/            # React 소스 코드
├── public/         # 정적 자산 (favicon 등)
├── dist/           # (빌드 시 생성)
├── vite.config.ts  # Vite 설정
├── package.json
├── tsconfig.json
└── README.md       # ← 이 파일
```

---

## 자주 묻는 질문(FAQ)

### Q. 대시보드에서 Pod 리스트가 안 보입니다!
A. 백엔드에서 podman 정보를 제공하는 API 서버가 정상적으로 실행 중이어야 합니다.  
(예: `podman ps` 명령을 래핑하는 Express 서버 등)  

### Q. package 설치 중 오류가 발생합니다.
A. Node.js(권장 18 이상)와 npm(권장 9 이상)이 설치되어 있는지 확인하세요.

### Q. 포트(5173)가 이미 사용 중이라고 나옵니다.
A. 이미 떠 있는 Vite 개발 서버가 있다면 종료하거나,  
`vite.config.ts`에서 다른 포트로 변경하세요.

---

