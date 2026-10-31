# RPCRad

[简体中文](README.md) | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

Rust, GPUI, GPUI-KIT 0.8로 만든 로컬 gRPC 데스크톱 디버깅 도구입니다. 실제 요청은 앱에 포함된 grpcurl 1.9.3을 사용합니다. WebView 없이 네이티브 UI로 실행됩니다.

## 설치 및 업데이트

[GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest)에서 운영체제와 CPU에 맞는 패키지를 다운로드하세요.

| 운영체제 | 아키텍처 | 패키지 |
| --- | --- | --- |
| macOS 15 이상 | Apple Silicon / Intel | 각각의 `.dmg` |
| Windows 10/11 | x64 | `-setup.exe` 설치 프로그램 또는 휴대용 `.zip` |
| Ubuntu 24.04 이상 | x64 | `.deb` 또는 `.tar.gz` |
| Omarchy / Arch Linux | x64 | `.pkg.tar.zst`, Wayland 및 X11 |

패키지에는 grpcurl, 예제 Proto, 의존성 라이선스가 포함됩니다. 앱 실행에 Rust나 Go를 따로 설치할 필요는 없습니다. Linux에는 정상적인 Vulkan 드라이버와 데스크톱 portal이 필요합니다. 한국어 등의 표시를 위해 `noto-fonts-cjk` 같은 글꼴도 설치하세요. Windows/Linux ARM64 패키지는 제공하지 않습니다.

Ubuntu는 `sudo apt install ./rpcrad_VERSION_amd64.deb`, Arch는 `sudo pacman -U ./rpcrad-VERSION-1-x86_64.pkg.tar.zst`로 설치합니다. VERSION을 실제 버전으로 바꾸세요. 휴대용 패키지는 전체 압축을 풀고 실행 파일, grpcurl, resources를 함께 유지해야 합니다. 자세한 내용은 [설치 안내(중국어)](docs/INSTALL.md)를 참고하세요.

macOS에서는 시스템 메뉴의 **RPCRad → RPCRad 정보 / 업데이트 확인**, Windows/Linux에서는 앱 상단 **RPCRad** 메뉴를 사용합니다. 시작할 때도 업데이트를 확인합니다. GitHub Releases, `latest.json`, SHA-256 방식으로 새 버전을 찾으며, 사용자가 직접 다운로드하고 설치합니다. 작업 공간은 유지되고 앱을 백그라운드에서 자동 교체하지 않습니다.

macOS 패키지는 아직 Developer ID 서명과 공증을 받지 않았으며 Windows 설치 프로그램도 Authenticode 서명이 없습니다. macOS 격리 때문에 실행할 수 없다면 공식 저장소에서 받은 파일인지, SHA-256이 일치하는지 확인한 뒤 필요한 경우에만 실행하세요.

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

다른 위치에 설치했다면 경로를 바꾸세요. 이 명령은 업데이트 명령이 아닌 선택적인 문제 해결 방법이며 앱이 자동 실행하지 않습니다.

## 인터페이스 언어

**RPCRad → 설정… → 언어**에서 **简体中文 / English / 日本語 / 한국어 / Français / Deutsch / Русский**를 즉시 전환할 수 있습니다. 선택은 저장되어 재시작 후에도 유지됩니다. 새 작업 공간과 언어 설정이 없는 기존 작업 공간의 기본값은 **자동 감지 (시스템)**입니다. 시작하거나 자동 감지를 선택할 때 시스템 우선순위에 따라 지원 언어를 선택합니다. 지역별 설정은 해당 언어로, 중국어는 간체로 표시합니다. 일치하는 언어가 없거나 감지에 실패하면 영어를 사용합니다. 기존 수동 선택은 유지하며 자동 모드에서는 감지된 언어도 표시합니다. `--language=auto`로도 활성화할 수 있습니다. `--language=ko`처럼 실행 인수로 `zh-CN`, `en`, `ja`, `ko`, `fr`, `de`, `ru`를 지정할 수도 있습니다.

설정은 별도 창에서 열리며 **시스템 설정 따르기(기본값) / 밝게 / 어둡게** 테마를 제공합니다. 변경은 즉시 적용되고 자동 저장되며, 시스템 설정을 따르면 OS의 테마 변경도 반영됩니다. 업데이트 확인과 앱 정보도 별도 창을 사용합니다. 다시 열면 기존 창이 앞으로 오며 요청 화면을 대체하지 않습니다. 설정 열기: macOS **⌘ ,**, Windows/Linux **Ctrl+,**. 보조 창 닫기: **Esc** 또는 **⌘/Ctrl+W**.

앱 문구와 기본 플러그인 설명은 번역됩니다. 사용자가 정한 이름, 변수 값, 요청과 응답 데이터, 기존 호출 스냅샷은 변경하지 않습니다. 서버 메시지나 번역이 없는 외부 플러그인 문구는 자동 번역하지 않습니다.

## 기능과 첫 요청

Proto를 가져오거나 서비스 리플렉션을 사용하여 메서드를 선택하고 서버 주소를 입력한 뒤 전송하세요. macOS에서는 **⌘ Enter**, Windows/Linux에서는 **Ctrl+Enter**로 전송합니다. **⌘/Ctrl+K**는 검색, **⌘/Ctrl+S**는 저장입니다. 사이드바와 편집기 열의 너비를 조절할 수 있습니다.

- 서비스 및 메서드 트리, 검색, 즐겨찾기, 탭, 요청 초안 저장.
- JSON 구문 강조, 서식 정리, Proto 기반 예제 생성.
- Unary 및 서버 스트리밍, 취소, 제한 시간, 상태와 소요 시간.
- Metadata, Bearer 토큰, 시스템 CA를 사용하는 TLS 검증.
- 환경과 전역 변수, 요청 헤더 서명 플러그인.
- 응답 본문, Headers, Trailers, 복사, 세션당 최대 50개 호출 기록.

기록을 열면 호출 당시의 본문, 연결 설정, 처리 전 헤더와 grpcurl에 전달된 최종 Metadata를 볼 수 있습니다. 일반 헤더는 그대로 표시하고 민감한 필드만 각각 표시/숨기기를 제공합니다. 복사는 현재 표시 상태를 따릅니다. 휴지통으로 기록을 지워도 진행 중인 호출은 중단되지 않습니다. **요청 불러오기**는 주소, 본문, 연결 설정을 복원하지만 헤더는 현재 설정을 사용하고 플러그인은 다음 전송 때 다시 계산합니다.

전송 전 실패나 취소는 전송되지 않은 것으로 명시합니다. 기록에는 HTTP/2가 자동 추가한 헤더가 없으며, 기록이 있다는 사실만으로 서버 수신을 보장하지 않습니다. 클라이언트/양방향 스트리밍, 사용자 CA/mTLS, 완전한 Proto import-root 관리는 아직 지원하지 않습니다. 응답 표시는 최대 8 MiB입니다.

## 환경 및 변수

환경 메뉴와 **변수**에서 환경을 만들고 이름을 변경하거나 삭제하며 값을 편집합니다. 활성 환경은 같은 이름의 전역 변수보다 우선합니다. 환경을 선택하지 않으면 전역 변수만 사용합니다.

주소에 `{{endpoint}}`, 플러그인 AppID에 `{{appid}}`, Secret에 `{{secret}}`를 사용할 수 있습니다. 요청 본문 예시:

```text
{
  "user_id": "{{user_id}}",
  "include_profile": {{include_profile}}
}
```

따옴표 안의 참조는 JSON 문자열로 이스케이프됩니다. 따옴표가 없는 참조는 유효한 JSON 숫자, 불리언, 객체 또는 배열이어야 합니다. Metadata 값은 문자열이어야 합니다. `{{host}}:{{port}}`와 같은 중첩 참조도 지원합니다. 정의되지 않은 변수, 중복 이름, 순환 참조가 있으면 전송을 막습니다. 이름은 영문자나 밑줄로 시작하고 숫자, 점, 하이픈을 포함할 수 있습니다.

일반 값은 저장됩니다. 민감한 변수 값, Secret 원문, 인증 정보, Metadata, 기록은 현재 세션에만 유지됩니다. 완전한 `{{secret}}` 참조는 저장되지만 민감한 값은 재시작 후 다시 입력해야 합니다. 민감한 변수를 요청 본문이나 헤더에 사용하면 실제로 전송되고 해당 세션의 호출 스냅샷에도 나타납니다. 디스크의 초안은 템플릿 상태로 유지됩니다.

## 요청 헤더 플러그인

**MD5 · AppID + 타임스탬프**를 선택하고 AppID와 Secret을 입력한 뒤 활성화하세요. 기본값은 현재 Unix 밀리초를 `ts`로 사용하고, `MD5(secret + appid + ts + secret)`의 소문자 16진수를 `sign`으로 계산하여 `appid`, `ts`, `sign`을 전송합니다. 서명과 헤더는 동일한 시각을 사용합니다. 헤더 이름, 시간 단위, 대소문자를 바꿀 수 있습니다.

HMAC-SHA256과 AES-256-GCM 예제도 포함됩니다. 작업 공간의 호출마다 선택한 플러그인 하나만 실행하며, 연결 실행이나 메서드별 지정은 지원하지 않습니다. 플러그인을 바꾸면 Secret은 지워지고 일반 설정은 플러그인별로 저장됩니다. 미리보기는 스냅샷이며 실제 전송 시 다시 계산합니다. 전송 결과는 새 호출 기록에서 확인하세요. [플러그인 개발 안내(중국어)](plugins/README.md)에 Rhai 및 번역 설정이 설명되어 있습니다.

## 개발 및 검증

Rust stable, Go, Python 3.11 이상과 운영체제별 네이티브 빌드 도구가 필요합니다. Go 버전은 `.go-version`에 고정되어 있습니다.

```sh
python tools/setup.py
cargo run --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python -m unittest discover -s tools/tests
```

`tools/bin/rpcrad-demo`를 실행하면 `127.0.0.1:50051`에서 예제 서버가 시작됩니다. GetUser, WatchUsers, `user_id = missing`의 NotFound 응답을 시험할 수 있습니다. macOS의 `Launch RPCRad.command`는 예제 서버와 앱을 엽니다. 계정이나 클라우드가 필요하지 않습니다. 시작 시 GitHub에 업데이트를 확인하지만 요청 데이터는 업로드하지 않습니다.

릴리스는 Mac 두 아키텍처, Windows, Ubuntu, Arch/Wayland에서 검증합니다. 매 릴리스 설명에는 중국어와 영어 설명, xattr 안내를 유지합니다. [릴리스 절차(중국어)](docs/RELEASING.md), [AGENTS.md](AGENTS.md)를 참고하세요. 주요 코드는 `src/ui.rs`, `src/rpc.rs`, 번역은 `locales/`, 예제는 `examples/protos/`, `tools/demo/`에 있습니다. 설계 참고: BloomRPC, Kreya, Yaak. 의존성 라이선스는 각 원본 프로젝트를 따릅니다.
