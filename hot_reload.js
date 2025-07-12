/**
 * Hot Reload for WGPU Viewer - Highly Optimized Vanilla JavaScript
 * Uses smart polling with conditional requests for maximum efficiency
 * Works on any static hosting (GitHub Pages, Netlify, Vercel, etc.)
 */

class GeometryHotReload {
    constructor(jsonPath = 'assets/sample_geometry.json', options = {}) {
        this.jsonPath = jsonPath;
        this.options = {
            minInterval: 100,      // Minimum polling interval (ms) - MUCH FASTER!
            maxInterval: 2000,     // Maximum polling interval (ms) - Reduced for faster response
            backoffMultiplier: 1.2, // Smaller backoff multiplier for quicker recovery
            resetAfterChange: true, // Reset to fast polling after change
            useVisibilityAPI: true, // Pause when tab is hidden
            useConditionalRequests: true, // Use If-Modified-Since headers
            fastMode: true,        // Enable ultra-fast mode for development
            ...options
        };
        
        this.currentInterval = this.options.minInterval;
        this.lastModified = null;
        this.lastETag = null;
        this.isPolling = false;
        this.reloadCallback = null;
        this.consecutiveUnchanged = 0;
        this.isVisible = true;
        
        this.setupVisibilityAPI();
        console.log('🔄 Optimized Hot Reload initialized for', jsonPath);
    }
    
    /**
     * Set callback function to call when geometry should be reloaded
     * @param {Function} callback - Function to call on geometry change
     */
    setReloadCallback(callback) {
        this.reloadCallback = callback;
    }
    
    /**
     * Start polling for file changes
     */
    start() {
        if (this.isPolling) {
            console.log('🔄 Hot reload already running');
            return;
        }
        
        this.isPolling = true;
        console.log(`🔄 Starting hot reload polling every ${this.pollInterval}ms`);
        
        // Initial load
        this.checkForChanges();
        
        // Set up polling interval
        this.intervalId = setInterval(() => {
            this.checkForChanges();
        }, this.pollInterval);
    }
    
    /**
     * Stop polling for file changes
     */
    stop() {
        if (!this.isPolling) {
            return;
        }
        
        this.isPolling = false;
        if (this.intervalId) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }
        console.log('🔄 Hot reload stopped');
    }
    
    /**
     * Check if the JSON file has changed (with adaptive polling)
     */
    async checkForChanges() {
        try {
            // Prepare headers for conditional request
            const headers = {
                'Cache-Control': 'no-cache, no-store, must-revalidate',
                'Pragma': 'no-cache',
                'Expires': '0'
            };
            
            // Add conditional request headers if available
            if (this.options.useConditionalRequests) {
                if (this.lastModified) {
                    headers['If-Modified-Since'] = this.lastModified;
                }
                if (this.lastETag) {
                    headers['If-None-Match'] = this.lastETag;
                }
            }
            
            const response = await fetch(this.jsonPath, {
                cache: 'no-cache',
                headers
            });
            
            // Handle 304 Not Modified (file unchanged)
            if (response.status === 304) {
                this.handleNoChange();
                return;
            }
            
            if (!response.ok) {
                console.warn(`🔄 Failed to fetch ${this.jsonPath}: ${response.status}`);
                this.handleNoChange();
                return;
            }
            
            // Get headers for future conditional requests
            const lastModified = response.headers.get('Last-Modified');
            const etag = response.headers.get('ETag');
            const content = await response.text();
            
            // Check if content actually changed
            const hasChanged = this.hasContentChanged(content, lastModified, etag);
            
            if (hasChanged) {
                console.log('🔄 Geometry file changed, reloading...');
                this.handleFileChanged(content, lastModified, etag);
            } else {
                this.handleNoChange();
            }
            
        } catch (error) {
            console.warn('🔄 Error checking for changes:', error.message);
        }
    }
    
    /**
     * Check if content has changed since last check
     */
    hasContentChanged(content, lastModified) {
        // First time loading
        if (this.lastContent === null) {
            return true;
        }
        
        // Compare by Last-Modified header if available
        if (lastModified && this.lastModified !== lastModified) {
            return true;
        }
        
        // Fallback: Compare content directly
        if (content !== this.lastContent) {
            return true;
        }
        
        return false;
    }
    
    /**
     * Get simple hash of content for comparison
     */
    simpleHash(str) {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash; // Convert to 32-bit integer
        }
        return hash;
    }
}

// Global hot reload instance
let geometryHotReload = null;

/**
 * Initialize hot reload for WGPU geometry
 * Call this after your WASM module is loaded
 */
function initGeometryHotReload(reloadFunction) {
    if (geometryHotReload) {
        geometryHotReload.stop();
    }
    
    geometryHotReload = new GeometryHotReload();
    geometryHotReload.setReloadCallback(reloadFunction);
    geometryHotReload.start();
    
    // Add visual indicator
    addHotReloadIndicator();
}

/**
 * Add visual indicator to show hot reload is active
 */
function addHotReloadIndicator() {
    // Remove existing indicator if present
    const existing = document.getElementById('hot-reload-indicator');
    if (existing) {
        existing.remove();
    }
    
    // Create new indicator
    const indicator = document.createElement('div');
    indicator.id = 'hot-reload-indicator';
    indicator.innerHTML = '🔄 Hot Reload Active';
    indicator.style.cssText = `
        position: fixed;
        top: 10px;
        right: 10px;
        background: #2196F3;
        color: white;
        padding: 8px 12px;
        border-radius: 6px;
        font-size: 12px;
        font-family: monospace;
        z-index: 10000;
        box-shadow: 0 2px 8px rgba(0,0,0,0.3);
        cursor: pointer;
        transition: all 0.3s ease;
    `;
    
    // Add click handler to toggle hot reload
    indicator.addEventListener('click', () => {
        if (geometryHotReload && geometryHotReload.isPolling) {
            geometryHotReload.stop();
            indicator.innerHTML = '⏸️ Hot Reload Paused';
            indicator.style.background = '#FF9800';
        } else if (geometryHotReload) {
            geometryHotReload.start();
            indicator.innerHTML = '🔄 Hot Reload Active';
            indicator.style.background = '#2196F3';
        }
    });
    
    document.body.appendChild(indicator);
}

// Export for use in modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { GeometryHotReload, initGeometryHotReload };
}
