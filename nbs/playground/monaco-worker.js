import {start} from 'https://esm.sh/monaco-editor-core@0.57.0/esm/vs/editor/editor.worker.start?bundle';
self.onmessage = () => start(() => ({}));
