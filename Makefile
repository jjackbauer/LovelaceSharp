# LovelaceSharp — Makefile
# Targets: build, run, runner, studio, test, clean, help
#
# `build`, `runner`, and `studio` publish Native AOT binaries by default
# (single-file, self-contained, no JIT warm-up). Native AOT requires the C++
# build tools (MSVC on Windows, clang on macOS/Linux).

PROJECT        := Lovelace.Console/Lovelace.Console.csproj
RUN_PROJECT    := Lovelace.Run/Lovelace.Run.csproj
STUDIO_PROJECT := Lovelace.Studio/Lovelace.Studio.csproj
CONFIGURATION  := Release
FRAMEWORK      := net10.0
PUBLISH_DIR    := Lovelace.Console/bin/$(CONFIGURATION)/$(FRAMEWORK)/publish
RUN_DIR        := Lovelace.Run/bin/$(CONFIGURATION)/$(FRAMEWORK)/publish
STUDIO_DIR     := Lovelace.Studio/bin/$(CONFIGURATION)/$(FRAMEWORK)/aot

# Native AOT publish flags.
AOT_FLAGS      := -p:PublishAot=true -p:InvariantGlobalization=true

# Detect OS for binary extension
ifeq ($(OS),Windows_NT)
    BINARY        := $(PUBLISH_DIR)/Lovelace.Console.exe
    STUDIO_BINARY := $(STUDIO_DIR)/Lovelace.Studio.exe
else
    BINARY        := $(PUBLISH_DIR)/Lovelace.Console
    STUDIO_BINARY := $(STUDIO_DIR)/Lovelace.Studio
endif

.PHONY: all build run runner studio test clean help

all: build

## build: Publish the console app (REPL) as a Native AOT binary.
build:
	dotnet publish $(PROJECT) \
		--configuration $(CONFIGURATION) \
		--framework $(FRAMEWORK) \
		$(AOT_FLAGS) \
		--output $(PUBLISH_DIR)

## run: Run the previously built console binary (requires `make build` first).
run: $(BINARY)
	$(BINARY)

## runner: Publish the non-interactive script runner (Lovelace.Run) as a Native AOT binary.
runner:
	dotnet publish $(RUN_PROJECT) \
		--configuration $(CONFIGURATION) \
		--framework $(FRAMEWORK) \
		$(AOT_FLAGS) \
		--output $(RUN_DIR)

## studio: Publish the Lovelace.Studio web IDE as a Native AOT binary and run it (binds to localhost).
studio:
	dotnet publish $(STUDIO_PROJECT) \
		--configuration $(CONFIGURATION) \
		--framework $(FRAMEWORK) \
		$(AOT_FLAGS) \
		--output $(STUDIO_DIR)
	$(STUDIO_BINARY) --contentRoot $(abspath $(STUDIO_DIR))

## test: Run the fast test suites (skips the slow Lovelace.Real.Tests).
test:
	dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj
	dotnet test Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj
	dotnet test Lovelace.Natural.Tests/Lovelace.Natural.Tests.csproj
	dotnet test Lovelace.Integer.Tests/Lovelace.Integer.Tests.csproj
	dotnet test Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj

$(BINARY):
	@echo "Binary not found — run 'make build' first."
	@exit 1

## clean: Remove all build and publish artifacts.
clean:
	dotnet clean $(PROJECT) --configuration $(CONFIGURATION)
	dotnet clean $(STUDIO_PROJECT)
	@if exist "$(PUBLISH_DIR)" rmdir /s /q "$(PUBLISH_DIR)" 2>nul || rm -rf "$(PUBLISH_DIR)"
	@if exist "$(STUDIO_DIR)" rmdir /s /q "$(STUDIO_DIR)" 2>nul || rm -rf "$(STUDIO_DIR)"

## help: List available targets.
help:
	@echo LovelaceSharp - targets:
	@echo   make build    Publish the console REPL as a Native AOT binary
	@echo   make run      Run the previously built console binary
	@echo   make runner   Publish the script runner (Lovelace.Run) as a Native AOT binary
	@echo   make studio   Publish + run the Lovelace.Studio web IDE as a Native AOT binary
	@echo   make test     Run the fast test suites
	@echo   make clean    Remove build artifacts
