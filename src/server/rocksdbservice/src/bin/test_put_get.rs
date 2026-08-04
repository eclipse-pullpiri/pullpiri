/*
 * Interactive PUT/GET test client with custom word input
 * Creates keys in format: helloworld_(word)_number
 */

use common::kvstore;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the service URL for testing
    env::set_var("ROCKSDB_SERVICE_URL", "http://localhost:47007");

    println!("🚀 RocksDB 인터랙티브 PUT/GET 테스트");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Give the service a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test health check first
    println!("\n🏥 Health Check...");
    match kvstore::health_check().await {
        Ok(is_healthy) => {
            if is_healthy {
                println!("✅ RocksDB 서비스 정상 동작 중!");
            } else {
                println!("⚠️ RocksDB 서비스 상태 불량");
                return Ok(());
            }
        }
        Err(e) => {
            println!("❌ Health check 실패: {}", e);
            return Ok(());
        }
    }

    // Get user input for custom word
    print!("\n💬 사용자 정의 단어를 입력하세요 (예: test, demo, sample): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("입력을 읽을 수 없습니다");
    let user_word = input.trim().to_string();

    if user_word.is_empty() {
        println!("❌ 빈 단어는 사용할 수 없습니다. 프로그램을 종료합니다.");
        return Ok(());
    }

    // Get number of items to test
    print!("🔢 생성할 아이템 개수를 입력하세요 (1-100): ");
    io::stdout().flush().unwrap();

    let mut count_input = String::new();
    io::stdin()
        .read_line(&mut count_input)
        .expect("입력을 읽을 수 없습니다");
    let count: usize = count_input.trim().parse().unwrap_or(5);

    if count == 0 || count > 100 {
        println!("❌ 1-100 범위의 숫자를 입력해주세요. 기본값 5를 사용합니다.");
    }
    let final_count = if count == 0 || count > 100 { 5 } else { count };

    println!("\n📋 테스트 설정:");
    println!("   🏷️  사용자 단어: {}", user_word);
    println!("   🔢 생성 개수: {}", final_count);
    println!("   📝 키 패턴: helloworld_{}_1~{}", user_word, final_count);

    // Phase 1: PUT operations
    println!("\n🔄 1단계: PUT 테스트 시작");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut put_success = 0;
    let mut put_errors = 0;

    for i in 1..=final_count {
        let scenario_key = format!("Scenario/helloworld_scenario_{}_{}", user_word, i);
        let package_key = format!("Package/helloworld_{}_{}", user_word, i);
        let model_key = format!("Model/helloworld_m_{}_{}", user_word, i);

        // Create YAML content
        let scenario_yaml = format!(
            "apiVersion: v1\nkind: Scenario\nmetadata:\n  name: helloworld_scenario_{}_{}\nspec:\n  condition: null\n  action: update\n  target: helloworld_{}_{}",
            user_word, i, user_word, i
        );

        let package_yaml = format!(
            "apiVersion: v1\nkind: Package\nmetadata:\n  label: null\n  name: helloworld_{}_{}\nspec:\n  pattern:\n    - type: plain\n  models:\n    - name: helloworld_m_{}_{}\n      node: yh\n      resources:\n        volume:\n        network:",
            user_word, i, user_word, i
        );

        let model_yaml = format!(
            "apiVersion: v1\nkind: Model\nmetadata:\n  name: helloworld_m_{}_{}\n  annotations:\n    io.pullpiri.annotations.package-type: helloworld\n    io.pullpiri.annotations.package-name: helloworld_{}_{}\n    io.pullpiri.annotations.package-network: default\n  labels:\n    app: helloworld_m_{}_{}app\nspec:\n  hostNetwork: true\n  containers:\n    - name: helloworld_c_{}_{}\n      image: quay.io/podman/hello:latest\n  terminationGracePeriodSeconds: 0\n  restartPolicy: Always",
            user_word, i, user_word, i, user_word, i, user_word, i
        );

        println!("\n🎯 PUT #{}: helloworld_{}_{}", i, user_word, i);

        // PUT Scenario
        match kvstore::put(&scenario_key, &scenario_yaml).await {
            Ok(()) => {
                put_success += 1;
                println!("   ✅ Scenario PUT 성공");
            }
            Err(e) => {
                put_errors += 1;
                println!("   ❌ Scenario PUT 실패: {}", e);
            }
        }

        // PUT Package
        match kvstore::put(&package_key, &package_yaml).await {
            Ok(()) => {
                put_success += 1;
                println!("   ✅ Package PUT 성공");
            }
            Err(e) => {
                put_errors += 1;
                println!("   ❌ Package PUT 실패: {}", e);
            }
        }

        // PUT Model
        match kvstore::put(&model_key, &model_yaml).await {
            Ok(()) => {
                put_success += 1;
                println!("   ✅ Model PUT 성공");
            }
            Err(e) => {
                put_errors += 1;
                println!("   ❌ Model PUT 실패: {}", e);
            }
        }
    }

    println!("\n📊 PUT 테스트 결과:");
    println!("   ✅ 성공: {}개", put_success);
    println!("   ❌ 실패: {}개", put_errors);

    // Phase 2: GET operations
    println!("\n🔍 2단계: GET 테스트 시작");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut get_success = 0;
    let mut get_errors = 0;

    for i in 1..=final_count {
        let scenario_key = format!("Scenario/helloworld_scenario_{}_{}", user_word, i);
        let package_key = format!("Package/helloworld_{}_{}", user_word, i);
        let model_key = format!("Model/helloworld_m_{}_{}", user_word, i);

        println!("\n🎯 GET #{}: helloworld_{}_{}", i, user_word, i);

        // GET Scenario
        match kvstore::get(&scenario_key).await {
            Ok(value) => {
                get_success += 1;
                println!("   ✅ Scenario GET 성공 ({} bytes)", value.len());
            }
            Err(e) => {
                get_errors += 1;
                println!("   ❌ Scenario GET 실패: {}", e);
            }
        }

        // GET Package
        match kvstore::get(&package_key).await {
            Ok(value) => {
                get_success += 1;
                println!("   ✅ Package GET 성공 ({} bytes)", value.len());
            }
            Err(e) => {
                get_errors += 1;
                println!("   ❌ Package GET 실패: {}", e);
            }
        }

        // GET Model
        match kvstore::get(&model_key).await {
            Ok(value) => {
                get_success += 1;
                println!("   ✅ Model GET 성공 ({} bytes)", value.len());
            }
            Err(e) => {
                get_errors += 1;
                println!("   ❌ Model GET 실패: {}", e);
            }
        }
    }

    // Show sample data
    if final_count > 0 {
        println!("\n📖 샘플 데이터 전체 내용 (Scenario #1):");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        let sample_key = format!("Scenario/helloworld_scenario_{}_1", user_word);
        match kvstore::get(&sample_key).await {
            Ok(value) => {
                println!("{}", value);
            }
            Err(e) => {
                println!("❌ 샘플 데이터 조회 실패: {}", e);
            }
        }
    }

    // Final summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 최종 테스트 결과 요약:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔧 설정:");
    println!("   📝 사용자 단어: {}", user_word);
    println!("   🔢 테스트 개수: {}", final_count);
    println!("   🎯 총 연산: {}개 (PUT + GET)", (final_count * 3 * 2));

    println!("\n📊 PUT 결과:");
    println!("   ✅ 성공: {}개", put_success);
    println!("   ❌ 실패: {}개", put_errors);
    println!(
        "   📈 성공률: {:.1}%",
        if put_success + put_errors > 0 {
            (put_success as f64 / (put_success + put_errors) as f64) * 100.0
        } else {
            0.0
        }
    );

    println!("\n🔍 GET 결과:");
    println!("   ✅ 성공: {}개", get_success);
    println!("   ❌ 실패: {}개", get_errors);
    println!(
        "   📈 성공률: {:.1}%",
        if get_success + get_errors > 0 {
            (get_success as f64 / (get_success + get_errors) as f64) * 100.0
        } else {
            0.0
        }
    );

    let total_success = put_success + get_success;
    let total_errors = put_errors + get_errors;
    let overall_success_rate = if total_success + total_errors > 0 {
        (total_success as f64 / (total_success + total_errors) as f64) * 100.0
    } else {
        0.0
    };

    println!("\n🎯 전체 결과:");
    println!("   ✅ 총 성공: {}개", total_success);
    println!("   ❌ 총 실패: {}개", total_errors);
    println!("   📈 전체 성공률: {:.1}%", overall_success_rate);

    if overall_success_rate == 100.0 {
        println!("\n🎉 모든 테스트 완벽 성공! 🎉");
    } else if overall_success_rate >= 90.0 {
        println!("\n👍 대부분의 테스트 성공!");
    } else {
        println!("\n⚠️ 일부 테스트에서 문제가 발생했습니다.");
    }

    Ok(())
}
