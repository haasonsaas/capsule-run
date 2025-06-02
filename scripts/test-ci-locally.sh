#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if act is installed
if ! command -v act &> /dev/null; then
    print_error "act is not installed. Please install it first:"
    echo "  macOS: brew install act"
    echo "  Linux: curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash"
    echo "  Windows: choco install act-cli"
    exit 1
fi

# Check if Docker is running
if ! docker info &> /dev/null; then
    print_error "Docker is not running. Please start Docker first."
    exit 1
fi

print_status "🧪 Testing capsule-run GitHub Actions locally with act"

# Function to run act with error handling
run_act() {
    local description="$1"
    shift
    
    print_status "Running: $description"
    if act "$@" --reuse; then
        print_success "$description completed successfully"
    else
        print_error "$description failed"
        return 1
    fi
}

# Default to running basic tests
TEST_LEVEL="${1:-basic}"

case "$TEST_LEVEL" in
    "basic")
        print_status "🔍 Running basic CI tests (check, format, clippy)"
        run_act "Code formatting check" push -j fmt --dry-run
        run_act "Compilation check (no seccomp)" push -j check --matrix features:"--no-default-features"
        run_act "Clippy linting" push -j clippy
        print_success "Basic tests completed! ✅"
        ;;
        
    "test")
        print_status "🧪 Running test suite"
        run_act "Unit tests (no seccomp)" push -j test --matrix features:"--no-default-features"
        run_act "Integration tests" -W .github/workflows/comprehensive-test.yml -j cross-platform-test
        print_success "Test suite completed! ✅"
        ;;
        
    "security")
        print_status "🔒 Running security tests"
        run_act "Security audit" -W .github/workflows/security.yml -j audit
        run_act "Security linting" -W .github/workflows/security.yml -j clippy-security
        run_act "Security validation" -W .github/workflows/comprehensive-test.yml -j security-validation
        print_success "Security tests completed! ✅"
        ;;
        
    "comprehensive")
        print_status "🚀 Running comprehensive test suite"
        
        print_status "Phase 1: Basic validation"
        run_act "Code formatting" push -j fmt --dry-run
        run_act "Compilation check" push -j check --matrix features:"--no-default-features"
        
        print_status "Phase 2: Testing"
        run_act "Unit tests" push -j test --matrix features:"--no-default-features"
        
        print_status "Phase 3: Code quality"
        run_act "Clippy linting" push -j clippy
        
        print_status "Phase 4: Documentation"
        run_act "Documentation tests" -W .github/workflows/comprehensive-test.yml -j documentation-test
        
        print_status "Phase 5: Security"
        run_act "Security validation" -W .github/workflows/comprehensive-test.yml -j security-validation
        
        print_success "Comprehensive tests completed! ✅"
        ;;
        
    "feature-matrix")
        print_status "🔄 Testing feature combinations"
        run_act "Test with seccomp" push -j test --matrix features:""
        run_act "Test without seccomp" push -j test --matrix features:"--no-default-features"
        run_act "Test explicit seccomp" push -j test --matrix features:"--features seccomp"
        print_success "Feature matrix tests completed! ✅"
        ;;
        
    "cross-platform")
        print_status "🌐 Testing cross-platform compatibility"
        run_act "Ubuntu tests" push -j test --matrix os:"ubuntu-latest"
        run_act "macOS tests" push -j test-macos
        print_success "Cross-platform tests completed! ✅"
        ;;
        
    "quick")
        print_status "⚡ Running quick validation"
        run_act "Quick compilation check" push -j check --matrix features:"--no-default-features"
        run_act "Quick format check" push -j fmt --dry-run
        print_success "Quick validation completed! ✅"
        ;;
        
    "list")
        print_status "📋 Available workflows and jobs:"
        echo ""
        act -l
        ;;
        
    *)
        echo "Usage: $0 [test_level]"
        echo ""
        echo "Available test levels:"
        echo "  basic          - Code formatting, compilation, clippy (default)"
        echo "  test           - Run test suite"
        echo "  security       - Security tests and validation"
        echo "  comprehensive  - Full test suite (takes longer)"
        echo "  feature-matrix - Test all feature combinations"
        echo "  cross-platform - Test Linux and macOS compatibility"
        echo "  quick          - Fast validation checks"
        echo "  list           - List all available workflows and jobs"
        echo ""
        echo "Examples:"
        echo "  $0 basic                # Run basic checks"
        echo "  $0 test                 # Run tests"
        echo "  $0 comprehensive        # Run everything"
        echo "  $0 quick                # Fast check before commit"
        exit 1
        ;;
esac

print_success "🎉 Local testing completed successfully!"
echo ""
print_status "💡 Tips:"
echo "  - Run '$0 quick' before each commit"
echo "  - Run '$0 comprehensive' before releases"
echo "  - Use '$0 list' to see all available jobs"
echo "  - Add '--verbose' to act commands for debugging"