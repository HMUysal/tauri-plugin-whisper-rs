<script lang="ts">
  import { initialize, transcribeFromFile, release } from 'tauri-plugin-whisper-rs-api';
  import type { 
    InitializeRequest, 
    TranscriptionFileRequest, 
    TranscriptionResponse, 
    GenericResponse 
  } from 'tauri-plugin-whisper-rs-api';

  // Svelte 5 state management
  let modelPath: string = $state('ggml-base.bin'); 
  let audioPath: string = $state('Recording 1.flac'); 
  let language: string = $state('tr');
  let isLoading: boolean = $state(false);
  let isInitialized: boolean = $state(false);
  let logs: string = $state('');

  /**
   * Helper to append logs to the UI with timestamps.
   * @param {any} message - The message or object to log.
   */
  function addLog(message: any): void {
    const time: string = new Date().toLocaleTimeString();
    const content: string = typeof message === 'string' ? message : JSON.stringify(message, null, 2);
    logs = `[${time}] ${content}<br>` + logs;
  }

  /**
   * Step 1: Initialize the model
   */
  async function handleInit(): Promise<void> {
    if (isLoading) return;
    isLoading = true;
    addLog(`Initializing model: ${modelPath}...`);

    const payload: InitializeRequest = { modelPath };

    try {
      const result: GenericResponse = await initialize(payload);
      if (result.status) {
        isInitialized = true;
        addLog("Success: Model loaded into memory.");
      } else {
        addLog(`Init Error: ${result.message}`);
      }
    } catch (err) {
      addLog(`Init Failed: ${err}`);
    } finally {
      isLoading = false;
    }
  }

  /**
   * Step 2: Run transcription on a file
   */
  async function handleTranscribe(): Promise<void> {
    if (isLoading || !isInitialized) return;
    
    isLoading = true;
    addLog("Transcription process started...");

    const payload: TranscriptionFileRequest = {
      audioPath: audioPath,
      language: language === '' ? undefined : language,
      beamSize: 5 // Defaulting to 5 as per our Rust logic
    };

    try {
      const result: TranscriptionResponse = await transcribeFromFile(payload);
      
      if (result.error) {
        addLog(`Backend Error: ${result.error}`);
      } else {
        addLog(`Result Text: ${result.text}`);
      }
    } catch (err) {
      addLog(`Invoke Failed: ${err}`);
    } finally {
      isLoading = false;
    }
  }

  /**
   * Step 3: Release resources
   */
  async function handleRelease(): Promise<void> {
    if (isLoading) return;
    isLoading = true;

    try {
      const result: GenericResponse = await release();
      if (result.status) {
        isInitialized = false;
        addLog("Memory Released: Model and State dropped.");
      }
    } catch (err) {
      addLog(`Release Failed: ${err}`);
    } finally {
      isLoading = false;
    }
  }
</script>

<main class="container">
  <h1>Whisper RS Plugin Test (Svelte 5)</h1>

  <div class="card">
    <div class="input-group">
      <label for="model">Model Path:</label>
      <input id="model" bind:value="{modelPath}" placeholder="ggml-small.bin" />
    </div>

    <div class="input-group">
      <label for="audio">Audio Path:</label>
      <input id="audio" bind:value="{audioPath}" placeholder="audio.mp3" />
    </div>

    <div class="input-group">
      <label for="lang">Language:</label>
      <input id="lang" bind:value="{language}" placeholder="tr, en, auto..." />
    </div>

    <div class="actions">
      <!-- Logic split into steps to match our new Rust architecture -->
      <button onclick="{handleInit}" disabled="{isLoading || isInitialized}" class="init">
        {isInitialized ? 'Model Ready' : '1. Initialize Model'}
      </button>

      <button onclick="{handleTranscribe}" disabled="{isLoading || !isInitialized}" class="run">
        {isLoading ? 'Processing...' : '2. Run Transcription'}
      </button>

      <button onclick="{handleRelease}" disabled="{isLoading || !isInitialized}" class="release">
        3. Release Memory
      </button>
    </div>
  </div>

  <div class="console">
    <h3>Output & Logs</h3>
    <div class="log-view">
      {@html logs || 'Ready to test. Please initialize the model first.'}
    </div>
  </div>
</main>

<style>
  .container {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  .card {
    background: #2a2a2a;
    padding: 1.5rem;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
    margin-bottom: 2rem;
    color: white;
    box-shadow: 0 4px 20px rgba(0,0,0,0.3);
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  input {
    padding: 0.75rem;
    border-radius: 6px;
    border: 1px solid #444;
    background: #1a1a1a;
    color: #e0e0e0;
    transition: border-color 0.2s;
  }

  input:focus {
    border-color: #007bff;
    outline: none;
  }

  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 1rem;
    margin-top: 1rem;
  }

  button {
    padding: 0.8rem;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-weight: bold;
    transition: opacity 0.2s, transform 0.1s;
  }

  button:active { transform: scale(0.98); }
  button:disabled { background: #444 !important; cursor: not-allowed; opacity: 0.6; }

  .init { background: #28a745; }
  .run { background: #007bff; }
  .release { background: #dc3545; }

  .console {
    background: #111;
    color: #00ff00;
    padding: 1.5rem;
    border-radius: 8px;
    font-family: 'Fira Code', 'Courier New', monospace;
    font-size: 0.85rem;
    border: 1px solid #333;
  }

  .log-view {
    max-height: 400px;
    overflow-y: auto;
    line-height: 1.5;
  }
</style>