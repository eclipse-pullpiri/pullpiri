## 4.1 Scenario 정의 및 상태 전이 다이어그램
### 4.1.1 상태 정의
| 상태 | 설명 |
|------|------|
| idle | 시나리오가 초기화된 상태 (아직 활성화되지 않음) |
| waiting | 조건이 등록된 상태 |
| satisfied | 조건이 만족된 상태 |
| allowed | 정책에 의해 실행이 허용된 상태 |
| denied | 정책에 의해 실행이 거부된 상태 |
| completed | 시나리오 실행이 완료된 상태 |

### 4.1.2 상태 전이 다이어그램
```mermaid
stateDiagram-v2
  [*] --> idle: 생성
  idle --> waiting: 조건 등록
  waiting --> satisfied: 조건 만족
  satisfied --> allowed: 정책 검증 성공
  satisfied --> denied: 정책 검증 실패
  allowed --> completed: 시나리오 완료
```

## 2. package 상태 정의 및 상태 전이 조건 요약표
| 상태      | 설명 | 조건 |
|-----------|------|---------------------------------------------------|
| idle      | 맨 처음 package의 상태 | 생성 시 기본 상태 |
| paused    | 모든 model이 paused 상태일 때 | 모든 model이 paused 상태 |
| exited    | 모든 model이 exited 상태일 때 | 모든 model이 exited 상태 |
| degraded  | 일부 model이 dead 상태일 때 | 일부(1개 이상) model이 dead 상태, 단 모든 model이 dead가 아닐 때 |
| error     | 모든 model이 dead 상태일 때 | 모든 model이 dead 상태 |
| running   | 위 조건을 모두 만족하지 않을 때(기본 상태) | 위 조건을 모두 만족하지 않을 때(기본 상태) |

## 3. model 상태 정의 및 상태 전이 조건 요약표
| 상태      | 설명 | 조건 |
|-----------|------|---------------------------------------------------|
| Created   | model의 최초 상태 | 생성 시 기본 상태 |
| Paused    | 모든 container가 paused 상태일 때 | 모든 container가 paused 상태 |
| Exited    | 모든 container가 exited 상태일 때 | 모든 container가 exited 상태 |
| Dead      | 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패 | 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패 |
| Running   | 위 조건을 모두 만족하지 않을 때(기본 상태) | 위 조건을 모두 만족하지 않을 때(기본 상태) |

## 4. container 상태 정의 및 상태 전이 조건 요약표
| 상태     | 설명                                                                 | 조건                                                         |
|----------|----------------------------------------------------------------------|--------------------------------------------------------------|
| Created  | 아직 실행된 컨테이너가 없는 상태. 컨테이너가 생성되지 않았거나 모두 삭제된 경우 | 컨테이너가 생성되지 않았거나 모두 삭제된 경우                |
| Running  | 하나 이상의 컨테이너가 실행 중인 상태                                 | 하나 이상의 컨테이너가 실행 중                                |
| Stopped  | 하나 이상의 컨테이너가 중지되었고, 실행 중인 컨테이너는 없음          | 하나 이상의 컨테이너가 중지, 실행 중인 컨테이너는 없음        |
| Exited   | Pod 내 모든 컨테이너가 종료된 상태                                    | 모든 컨테이너가 종료됨                                       |
| Dead     | Pod의 상태 정보를 가져오는 데 실패한 경우 (메타데이터 손상, 시스템 오류 등) | 상태 정보 조회 실패, 시스템 오류 등                           |
