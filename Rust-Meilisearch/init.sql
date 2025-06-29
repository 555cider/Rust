-- 국가 정보를 저장하는 테이블
CREATE TABLE countries (
    code CHAR(2) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    CONSTRAINT unique_country_name UNIQUE (name)
);

-- 도시 정보를 저장하는 메인 테이블
CREATE TABLE cities (
    geonameid VARCHAR(20) PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    asciiname VARCHAR(200) NOT NULL,
    latitude DECIMAL(10, 8) NOT NULL,
    longitude DECIMAL(11, 8) NOT NULL,
    country_code CHAR(2) NOT NULL REFERENCES countries(code),
    population BIGINT NOT NULL DEFAULT 0,
    timezone VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_latitude CHECK (latitude BETWEEN -90 AND 90),
    CONSTRAINT valid_longitude CHECK (longitude BETWEEN -180 AND 180)
);

-- 도시의 대체 이름들을 저장하는 테이블
CREATE TABLE city_alternate_names (
    id SERIAL PRIMARY KEY,
    city_geonameid VARCHAR(20) REFERENCES cities(geonameid),
    alternate_name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 위도/경도에 대한 인덱스
CREATE INDEX idx_cities_location ON cities (latitude, longitude);

-- 도시 이름 검색을 위한 인덱스
CREATE INDEX idx_cities_name_btree ON cities USING btree (name);
CREATE INDEX idx_cities_asciiname_btree ON cities USING btree (asciiname);

-- 업데이트 시 updated_at 자동 갱신을 위한 트리거
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_cities_updated_at
    BEFORE UPDATE ON cities
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();