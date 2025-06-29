# Admin Server

관리자 대시보드 서버 (Rust + Axum + Tera)

## 주요 기능

- JWT 기반 사용자 인증
- 역할 기반 접근 제어 (RBAC)
- 관리자 대시보드
- RESTful API 엔드포인트
- 데이터베이스 마이그레이션
- 로깅 및 오류 처리

## 기술 스택

- **백엔드**: Rust + Axum 웹 프레임워크
- **데이터베이스**: SQLite with SQLx
- **템플릿 엔진**: Tera
- **인증**: JWT (JSON Web Tokens)
- **로깅**: tracing + tracing-subscriber
- **유효성 검증**: validator

## 시작하기

### 필수 사항

- Rust (최신 안정화 버전)
- SQLite3

### 설치 및 실행

1. 저장소 클론:
   ```bash
   git clone <repository-url>
   cd admin-server
   ```

2. 환경 변수 설정:
   ```bash
   cp .env.example .env
   # .env 파일을 수정하여 설정을 변경하세요
   ```

3. 데이터베이스 마이그레이션 실행:
   ```bash
   sqlx migrate run
   ```

4. 개발 서버 실행:
   ```bash
   cargo make --no-workspace start
   ```

## 프로젝트 구조

```
src/
├── errors/           # 커스텀 에러 타입 및 처리
├── filter/           # 미들웨어 및 요청 필터
├── handler/          # 요청 핸들러
│   ├── api/         # API 엔드포인트
│   └── view/        # 뷰 핸들러
├── model/            # 데이터 모델 및 DTO
├── repository/       # 데이터베이스 리포지토리
├── service/          # 비즈니스 로직
├── util/             # 유틸리티 함수
└── main.rs           # 애플리케이션 진입점
```

## 인증 시스템

JWT를 사용한 인증 시스템이 구현되어 있습니다. 인증 미들웨어는 다음과 같이 사용합니다:

```rust
use axum::middleware;

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(handler))
}
```

## 환경 변수

`.env` 파일에 다음 변수들을 설정해야 합니다:

```env
DATABASE_URL=sqlite:./data/db.sqlite
JWT_SECRET=your_jwt_secret_here
PORT=3000
```

## 개발

- 테스트 실행:
  ```bash
  cargo test
  ```

- 코드 포맷팅:
  ```bash
  cargo fmt
  ```

- 린트 체크:
  ```bash
  cargo clippy
  ```

## 라이선스

이 프로젝트는 MIT 라이선스 하에 배포됩니다.
