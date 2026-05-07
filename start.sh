#!/bin/bash

export LANG=zh_CN.UTF-8
export LC_ALL=zh_CN.UTF-8

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BINARY_NAME="biosphere-ai-lab-app"
BINARY_PATH="$SCRIPT_DIR/src-tauri/target/release/$BINARY_NAME"

LOG_DIR="$SCRIPT_DIR/data/logs"
LOG_FILE="$LOG_DIR/lab.log"

PID_FILE="$LOG_DIR/$BINARY_NAME.pid"

DEV_PORT=1428

log_info() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')] [INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')] [WARN]${NC} $1"
}

log_usage() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')] [USAGE]${NC} $1"
}

check_rust() {
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo 未安装或不在PATH中"
        exit 1
    fi
}

check_node() {
    if ! command -v pnpm &> /dev/null; then
        if ! command -v npm &> /dev/null; then
            log_error "pnpm/npm 未安装或不在PATH中"
            exit 1
        fi
    fi
}

create_directories() {
    mkdir -p "$LOG_DIR"
    mkdir -p "$SCRIPT_DIR/data"
}

build_project() {
    log_info "正在构建 Biosphere AI Lab..."

    cd "$SCRIPT_DIR"

    if [ ! -d "node_modules" ]; then
        log_info "安装前端依赖..."
        pnpm install
        if [ $? -ne 0 ]; then
            log_error "前端依赖安装失败"
            exit 1
        fi
    fi

    log_info "构建前端..."
    pnpm build
    if [ $? -ne 0 ]; then
        log_error "前端构建失败"
        exit 1
    fi

    log_info "构建 Rust 后端..."
    cd "$SCRIPT_DIR/src-tauri"
    LIBTORCH_USE_PYTORCH=1 LIBTORCH_BYPASS_VERSION_CHECK=1 cargo build --release
    if [ $? -ne 0 ]; then
        log_error "Rust 后端构建失败"
        exit 1
    fi

    if [ ! -f "$BINARY_PATH" ]; then
        log_error "可执行文件不存在: $BINARY_PATH"
        exit 1
    fi

    log_info "构建成功"
}

pre_check() {
    check_rust
    check_node
    create_directories
    build_project
}

check_and_stop_processes() {
    log_info "检查是否有冲突的进程..."

    LAB_PIDS=$(pgrep -f "$BINARY_NAME")
    if [ ! -z "$LAB_PIDS" ]; then
        log_warn "发现已运行的 Biosphere AI Lab 进程:"
        echo "$LAB_PIDS" | while read pid; do
            echo "  PID: $pid, 命令: $(ps -p $pid -o comm=)"
        done

        read -p "是否要停止这些进程? (y/n): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            log_info "正在停止已运行的进程..."
            echo "$LAB_PIDS" | xargs kill -TERM 2>/dev/null
            sleep 2

            REMAINING_PIDS=$(pgrep -f "$BINARY_NAME")
            if [ ! -z "$REMAINING_PIDS" ]; then
                log_warn "部分进程仍在运行，强制停止..."
                echo "$REMAINING_PIDS" | xargs kill -9 2>/dev/null
            fi

            log_info "已停止所有进程"
        else
            log_warn "继续启动可能会与现有进程冲突"
        fi
    fi

    DEV_PORT_PID=$(lsof -ti:${DEV_PORT} 2>/dev/null)
    if [ ! -z "$DEV_PORT_PID" ]; then
        log_warn "发现占用 ${DEV_PORT} 端口的进程:"
        lsof -i:${DEV_PORT} | while read line; do
            echo "  $line"
        done

        read -p "是否要停止占用 ${DEV_PORT} 端口的进程? (y/n): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            log_info "正在停止占用 ${DEV_PORT} 端口的进程..."
            kill -TERM $DEV_PORT_PID 2>/dev/null
            sleep 1

            REMAINING_PID=$(lsof -ti:${DEV_PORT} 2>/dev/null)
            if [ ! -z "$REMAINING_PID" ]; then
                log_warn "端口仍被占用，强制停止..."
                kill -9 $REMAINING_PID 2>/dev/null
            fi

            log_info "已停止占用 ${DEV_PORT} 端口的进程"
        else
            log_warn "继续启动可能会与现有进程冲突"
        fi
    fi
}

start_backend() {
    log_info "准备启动 Biosphere AI Lab..."

    pre_check

    export RUST_LOG=info
    export BIOSPHERE_AI_LAB_ROOT="$SCRIPT_DIR"
    export LIBTORCH_USE_PYTORCH=1
    export LIBTORCH_BYPASS_VERSION_CHECK=1

    log_info "正在启动 Biosphere AI Lab..."

    nohup "$BINARY_PATH" > "$LOG_FILE" 2>&1 &
    PID=$!

    echo $PID > "$PID_FILE"

    log_info "等待服务启动..."
    sleep 3

    if kill -0 $PID 2>/dev/null; then
        log_info "Biosphere AI Lab 已成功启动"
        log_info "服务 PID: $PID"
        log_info "日志文件: $LOG_FILE"
        log_info "PID 文件: $PID_FILE"
        log_info ""
        log_info "使用以下命令停止服务:"
        log_info "  kill $PID"
        log_info "  或"
        log_info "  kill \$(cat $PID_FILE)"
        log_info ""
        log_info "使用以下命令查看日志:"
        log_info "  tail -f $LOG_FILE"
        log_info ""
        log_info "服务已在后台运行，控制台将退出"
    else
        log_error "Biosphere AI Lab 启动失败"
        log_info "请检查日志: $LOG_FILE"
        rm -f "$PID_FILE"
        exit 1
    fi
}

watch_backend() {
    log_info "启动后端监控模式..."
    log_info "崩溃保护: 连续 ${MAX_CRASH_COUNT} 次崩溃后自动停止"
    reset_crash_count

    while true; do
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if ! kill -0 $PID 2>/dev/null; then
                log_error "检测到进程已退出"
                CRASH_COUNT=$(record_crash)

                if [ $CRASH_COUNT -ge $MAX_CRASH_COUNT ]; then
                    log_error "连续崩溃 ${MAX_CRASH_COUNT} 次，停止自动重启！"
                    log_error "请检查日志: $LOG_FILE"
                    reset_crash_count
                    exit 1
                fi

                log_warn "${CRASH_WINDOW}秒内第 ${CRASH_COUNT} 次崩溃，5秒后重启..."
                sleep 5
                start_backend
            fi
        else
            log_error "PID 文件不存在，尝试重新启动..."
            CRASH_COUNT=$(record_crash)

            if [ $CRASH_COUNT -ge $MAX_CRASH_COUNT ]; then
                log_error "连续崩溃 ${MAX_CRASH_COUNT} 次，停止自动重启！"
                reset_crash_count
                exit 1
            fi

            sleep 5
            start_backend
        fi

        sleep 5
    done
}

stop_backend() {
    log_info "停止 Biosphere AI Lab..."

    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 $PID 2>/dev/null; then
            log_info "发现运行中的服务 (PID: $PID)，正在停止..."
            kill -TERM $PID 2>/dev/null
            sleep 2

            if kill -0 $PID 2>/dev/null; then
                log_warn "正常停止失败，强制停止服务..."
                kill -9 $PID 2>/dev/null
            fi

            log_info "服务已停止"
        else
            log_warn "PID 文件存在，但进程未运行，清理 PID 文件"
        fi

        rm -f "$PID_FILE"
    else
        LAB_PIDS=$(pgrep -f "$BINARY_NAME")
        if [ ! -z "$LAB_PIDS" ]; then
            log_info "发现运行中的进程，正在停止..."
            echo "$LAB_PIDS" | xargs kill -TERM 2>/dev/null
            sleep 2

            REMAINING_PIDS=$(pgrep -f "$BINARY_NAME")
            if [ ! -z "$REMAINING_PIDS" ]; then
                log_warn "部分进程仍在运行，强制停止..."
                echo "$REMAINING_PIDS" | xargs kill -9 2>/dev/null
            fi

            log_info "所有进程已停止"
        else
            log_info "没有发现运行中的进程"
        fi
    fi
}

check_status() {
    log_info "检查 Biosphere AI Lab 服务状态..."

    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 $PID 2>/dev/null; then
            log_info "服务正在运行 (PID: $PID)"
            log_info "日志文件: $LOG_FILE"
        else
            log_warn "PID 文件存在，但进程未运行"
        fi
    else
        LAB_PIDS=$(pgrep -f "$BINARY_NAME")
        if [ ! -z "$LAB_PIDS" ]; then
            log_info "发现运行中的进程:"
            echo "$LAB_PIDS" | while read pid; do
                echo "  PID: $pid, 命令: $(ps -p $pid -o comm=)"
            done
        else
            log_info "没有发现运行中的进程"
        fi
    fi
}

MAX_CRASH_COUNT=3
CRASH_WINDOW=60
CRASH_MARKER="$LOG_DIR/.crash_count"

record_crash() {
    local now=$(date +%s)
    if [ -f "$CRASH_MARKER" ]; then
        local last_crash=$(cat "$CRASH_MARKER" | head -1)
        local count=$(cat "$CRASH_MARKER" | tail -1)
        local diff=$((now - last_crash))
        if [ $diff -lt $CRASH_WINDOW ]; then
            count=$((count + 1))
        else
            count=1
        fi
        echo "$now" > "$CRASH_MARKER"
        echo "$count" >> "$CRASH_MARKER"
        echo $count
    else
        echo "$now" > "$CRASH_MARKER"
        echo "1" >> "$CRASH_MARKER"
        echo 1
    fi
}

reset_crash_count() {
    rm -f "$CRASH_MARKER"
}

dev_mode() {
    log_info "以开发模式启动 Biosphere AI Lab..."
    log_info "前端开发服务器: http://localhost:${DEV_PORT}"
    log_info "崩溃保护: 连续 ${MAX_CRASH_COUNT} 次崩溃后自动停止"
    log_info "按 Ctrl+C 停止服务"
    echo "----------------------------------------"

    create_directories
    reset_crash_count

    cd "$SCRIPT_DIR"

    export RUST_LOG=info
    export BIOSPHERE_AI_LAB_ROOT="$SCRIPT_DIR"
    export LIBTORCH_USE_PYTORCH=1
    export LIBTORCH_BYPASS_VERSION_CHECK=1

    while true; do
        log_info "启动 tauri dev..."
        pnpm tauri dev 2>&1 | tee "$LOG_FILE"
        EXIT_CODE=${PIPESTATUS[0]}

        if [ $EXIT_CODE -eq 0 ]; then
            log_info "应用正常退出"
            reset_crash_count
            break
        fi

        CRASH_COUNT=$(record_crash)
        log_error "应用异常退出 (退出码: $EXIT_CODE, ${CRASH_WINDOW}秒内第 ${CRASH_COUNT} 次崩溃)"

        if [ $CRASH_COUNT -ge $MAX_CRASH_COUNT ]; then
            log_error "连续崩溃 ${MAX_CRASH_COUNT} 次，停止自动重启！"
            log_error "请检查日志: $LOG_FILE"
            log_error "修复后使用 './start.sh dev' 重新启动"
            reset_crash_count
            exit 1
        fi

        log_warn "等待 5 秒后重新启动..."
        sleep 5
    done
}

test_mode() {
    log_info "以测试模式启动 Biosphere AI Lab..."

    pre_check

    export RUST_LOG=debug
    export BIOSPHERE_AI_LAB_ROOT="$SCRIPT_DIR"
    export LIBTORCH_USE_PYTORCH=1
    export LIBTORCH_BYPASS_VERSION_CHECK=1

    log_info "正在以测试模式启动..."
    log_info "日志文件: $LOG_FILE"
    log_info "按 Ctrl+C 停止服务"
    echo "----------------------------------------"

    "$BINARY_PATH" 2>&1 | tee "$LOG_FILE"
}

clean_logs() {
    log_info "清理日志文件..."

    if [ -d "$LOG_DIR" ]; then
        rm -f "$LOG_DIR"/*.log
        log_info "应用日志已清理"
    fi

    if [ -d "$SCRIPT_DIR/src-tauri/logs" ]; then
        rm -f "$SCRIPT_DIR/src-tauri/logs"/*.log
        log_info "Tauri 日志已清理"
    fi

    log_info "日志清理完成"
}

show_logs() {
    if [ -f "$LOG_FILE" ]; then
        tail -f "$LOG_FILE"
    else
        log_error "日志文件不存在: $LOG_FILE"
    fi
}

show_help() {
    echo "Biosphere AI Lab 管理脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  start            启动服务（后台运行）"
    echo "  stop             停止服务"
    echo "  restart          重启服务"
    echo "  status           查看服务状态"
    echo "  dev              开发模式启动（前后端热重载）"
    echo "  test             测试模式启动（前台运行，debug 日志）"
    echo "  logs             实时查看日志"
    echo "  clean            清理日志文件"
    echo "  help, -h, --help 显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0 start         # 构建并启动服务（后台运行）"
    echo "  $0 dev           # 开发模式（前后端热重载）"
    echo "  $0 test          # 测试模式（前台运行，debug 日志）"
    echo "  $0 stop          # 停止服务"
    echo "  $0 restart       # 重启服务"
    echo "  $0 status        # 查看服务状态"
    echo "  $0 logs          # 实时查看日志"
    echo "  $0 clean         # 清理日志文件"
}

main() {
    case "$1" in
        start|"")
            log_info "Biosphere AI Lab 启动脚本"
            echo "========================================"

            check_and_stop_processes
            start_backend
            ;;
        stop)
            stop_backend
            ;;
        restart)
            log_info "Biosphere AI Lab 重启脚本"
            echo "========================================"

            stop_backend
            echo ""

            check_and_stop_processes
            start_backend
            ;;
        status)
            check_status
            ;;
        dev)
            log_info "Biosphere AI Lab 开发模式"
            echo "========================================"

            check_and_stop_processes
            dev_mode
            ;;
        test)
            log_info "Biosphere AI Lab 测试模式"
            echo "========================================"

            check_and_stop_processes
            test_mode
            ;;
        logs)
            show_logs
            ;;
        clean)
            clean_logs
            ;;
        help|-h|--help)
            show_help
            ;;
        *)
            log_error "未知选项: $1"
            log_usage "使用 '$0 help' 查看帮助信息"
            exit 1
            ;;
    esac
}

main "$@"
