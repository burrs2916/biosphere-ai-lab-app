# Biosphere AI Lab

A desktop application for machine learning model training, dataset management, and experiment tracking. Built with Tauri v2, Svelte 5, and Rust.

## Overview

Biosphere AI Lab provides a unified interface for the ML workflow: from dataset registration and quality analysis, through training configuration and execution, to model registration and evaluation. It runs as a native desktop application with a Rust backend and a Svelte-based frontend.

> **Status**: Early development (v0.1.0). The application currently uses a mock client for frontend development. The Rust backend is partially implemented — not all frontend features have working backend support yet.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | Svelte 5, SvelteKit, TypeScript, TailwindCSS, ECharts |
| Desktop | Tauri v2 |
| Backend | Rust |
| ML Engine | [Burn](https://github.com/tracel-ai/burn) (wGPU/CPU), [tch-rs](https://github.com/LaurentMazare/tch-rs) (LibTorch, optional) |
| Database | SQLite (via rusqlite) |
| Data Formats | CSV, JSON, Parquet, Arrow |

## Features

### Dataset Management
- Register datasets from local files (CSV, JSON, Parquet, Excel, Image, Text, etc.)
- Dataset listing with search, filtering (format, size, quality, status), and sorting
- Dataset detail view with column profiles, statistics, and quality scores
- Archive, restore, and delete datasets
- Dataset versioning with change notes and version diff
- Data preview with pagination
- Batch operations (archive, delete, restore)

### Data Quality & Analysis
- Quality scoring across dimensions: completeness, consistency, distribution, label quality, information density
- Health check panel with pass/warn/fail indicators
- Missing value heatmap visualization
- Column histograms and bar charts
- Data validation and integrity checks
- Training readiness assessment with recommendations
- Advanced analysis: correlation, drift detection, class imbalance, label quality (Confident Learning), feature leakage, split consistency, bias detection, slice analysis, influence analysis (TracIn, LOO)

### Data Workshop
- Interactive data preprocessing pipeline builder
- Preset templates: numeric cleaning, categorical encoding, full preprocessing
- Pipeline steps: normalize, standardize, one-hot encode, label encode, fill missing, drop missing
- Live data preview after preprocessing
- Register preprocessed data as a new dataset

### Training
- Step-by-step training configuration wizard (basic info → data config → model & hyperparameters → confirm)
- Model architectures: MLP, CNN (with more via plugin system)
- Task types: classification, regression, clustering, detection, segmentation, generation, custom
- Compute backends: CPU, CUDA, wGPU, Metal, ROCm
- Training lifecycle: start, stop, pause, resume, resume from checkpoint
- Real-time training progress with epoch/batch tracking and ETA
- Learning rate schedulers: constant, step decay, exponential decay, cosine annealing, linear decay
- Early stopping with configurable patience and monitoring metric
- Hardware-aware training recommendations

### Experiment Tracking
- Experiment list with status indicators, filtering, and grouping
- Detailed experiment view with metrics timeline (ECharts)
- Metric types: loss, accuracy, precision, recall, F1, MSE, RMSE, MAE, R2, AUC
- Experiment comparison (2-5 experiments) with parallel coordinates, scatter plots, and configuration diff
- Experiment cloning, tagging, archiving
- Artifact tracking and log viewing
- Batch experiment operations

### Model Management
- Model registration from completed experiments
- Model lifecycle: staging → production → archived
- Model versioning with descriptions and tags
- Model export: TorchScript, ONNX, BurnRecord
- Model serving with local inference endpoints (experimental)
- Model evaluation (classification and regression metrics)

### Hyperparameter Tuning
- Define hyperparameter search spaces (categorical, uniform, log-uniform, integer ranges)
- Tuning strategies: grid search, random search, Bayesian optimization
- Trial tracking with status and metric comparison

### Data Lineage
- Visual lineage graph showing relationships between datasets, experiments, and models
- Lineage tracing and impact analysis
- Mermaid diagram export

### Training Plans
- Multi-phase training plan creation with curriculum stages
- Data recipe builder with mixing strategies (concatenate, interleave, weighted, curriculum)
- Quality gates with configurable thresholds
- Budget configuration (time, compute, data limits)
- Plan validation and summarization
- Preset plan templates (pretraining, fine-tuning, instruction tuning, RLHF)

### Internationalization
- Full i18n support with Chinese (zh-CN) and English (en)
- Language switch in the top-right corner of the application
- All UI text, error messages, notifications, and time formatting are localized

### UI/UX
- Dark theme interface
- Keyboard shortcuts
- Toast notifications and notification stack
- Background task manager with progress tracking
- Status bar with connection status and refresh indicators
- Confirmation dialogs for destructive operations
- Empty state guides for new users

## Project Structure

```
biosphere-ai-lab-app/
├── src/                          # Frontend (Svelte)
│   ├── routes/
│   │   └── lab/
│   │       ├── +page.svelte      # Dashboard
│   │       ├── +layout.svelte    # Lab layout with navigation & status bar
│   │       ├── experiments/      # Experiment list & detail
│   │       ├── data/             # Dataset list, detail, workshop
│   │       ├── models/           # Model registry
│   │       ├── compare/          # Experiment comparison
│   │       ├── train/new/        # New training wizard
│   │       ├── plan/             # Training plan builder
│   │       ├── plans/            # Saved plans list
│   │       ├── tune/             # Hyperparameter tuning
│   │       ├── lineage/          # Data lineage graph
│   │       └── settings/         # Application settings
│   └── lib/
│       ├── i18n/                 # Internationalization (zh-CN, en)
│       └── lab/
│           ├── adapter/          # Backend client (Tauri + mock)
│           ├── components/       # Reusable UI components (25+)
│           ├── stores/           # Svelte stores (14 stores)
│           └── utils/            # Error localization & messages
├── src-tauri/                    # Tauri desktop shell (Rust)
│   └── src/
│       ├── lib.rs                # Tauri command registration
│       └── infrastructure/       # Config & logging
├── crates/
│   └── biosphere-ai-lab/         # Core Rust library
│       └── src/
│           ├── core/             # Config, events, plugins, sessions
│           ├── data/             # Dataset management, quality, lineage, recipes
│           ├── domain/           # DDD: dataset, experiment, model, training aggregates
│           ├── engine/           # ML engines (Burn, tch-rs)
│           ├── gateway/          # Tauri adapter & command handlers
│           ├── model/            # Model architectures (MLP, CNN presets)
│           ├── task/             # Task types (classification, etc.)
│           ├── training/         # Training manager & metrics
│           └── infrastructure/   # SQLite persistence
└── build/                        # Static build output
```

## Architecture

The application follows a layered architecture:

1. **Frontend (Svelte 5)** — Reactive UI with Svelte stores for state management. Communicates with the backend via Tauri's `invoke` API.
2. **Tauri Adapter** — Bridges frontend commands to the Rust backend. Serializes/deserializes data between TypeScript and Rust.
3. **Core Rust Library** — Domain-driven design with aggregates for Dataset, Experiment, Model, and Training. Uses a command bus pattern for write operations and event sourcing for state changes.
4. **ML Engines** — Pluggable engine system. Burn for cross-platform GPU/CPU inference and training, tch-rs for LibTorch-based training (optional feature).
5. **Persistence** — SQLite for dataset, experiment, model, and settings storage.

## Getting Started

### Prerequisites

- Node.js 18+
- Rust 1.77+ (with cargo)
- Tauri v2 CLI
- For tch-rs engine: LibTorch 2.2+ (set `LIBTORCH` environment variable)

### Development (Frontend only, with mock data)

```bash
npm install
npm run dev
```

This starts the SvelteKit dev server with mock backend data. No Rust toolchain required.

### Development (Full desktop app)

```bash
npm install
npm run tauri:dev
```

This launches the Tauri desktop window with the Rust backend.

### Build

```bash
npm run build          # Frontend only (static site)
npm run tauri build    # Full desktop application
```

## Limitations

- The mock client simulates most backend operations. Not all features have working Rust backend implementations.
- The tch-rs engine requires a local LibTorch installation and is behind the `tch-engine` feature flag.
- Model serving is experimental and not production-ready.
- Some advanced data analysis features (drift detection, influence analysis, bias detection) return mock data.
- No authentication or multi-user support.

## License

MIT
