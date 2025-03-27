# APIServer

## 1. 목적 프롬프트
### 주요기능
API Server는 내,외부 API를 제공하며 Scenario 등록 및 준비 작업을 수행 한다.  
1. REST API 오픈하여 Piccolo Cloud 와 통신 또는 direct 로 scenario 를 받는 역할을 한다.  
2. yaml 형식 string 으로 들어온 시나리오를 파싱한다.
3. common 에 정의된 scenario, package 모듈등으로 파싱하여 struct로 생성한다.
4. 파싱 결과를 etcd 에 저장하고 grpc를 통해 filtergateway 로 전달한다.  

### 주요 데이터 흐름
1. API-Server로 부터 grpc로  struct Scenario를 전달 받는다.   
2. 전달받은 Scenario 정보를 파싱하고 manager로 전달한다.  
3. manager는 전달받은 내용을 Vehicle로 전달하여 DDS 토픽을 구독한다.  


## 2. 파일 정보

APIServer
├── main.rs
├── manager.rs
├── artifact
│   ├── mod.rs
│   └── data.rs
├── grpc
│   ├── mod.rs
│   └── sender.rs
├── route
│   ├── mod.rs
│   └── api.rs
└── importer
    ├── mod.rs
    └── parser
        ├── mod.rs
        ├── package.rs
        └── scenario.rs


- **main.rs** - manager initialize 수행
- **manager.rs** - 차량 신호 listener 등록, gRPC receiver 로부터 받은 condition 정보로 filter 생성
- **grpc/mod.rs**
- **grpc/sender.rs** - filtergateway 로 gRPC 메시지 전달
- **artifact/mod.rs** - 
- **artifact/data.rs** - etcd에 파싱된 결과를 저장하거나 불러오르는 함수 구현 
- **route/mod.rs** 
- **route/api.rs** 
- **importer/parser/mod.rs** 
- **importer/parser/package.rs** 
- **importer/parser/scenario.rs** 
  


## 3. 주요 API 정보

### API : Initialize
- **API Name**: Initialize
- **File**: main.rs
- **Type**: function
- **Parameters**:
- **Returns**:
- **Description**: manager 스레드 초기화 및 listener 생성 작업 진행.


## 4.참고(참조파일 먼저 입력필요)
 

## 5.코드 생성 이후 추가 요구사항
 