/**
 * Hot Reload for WGPU Viewer - Complete Optimized Vanilla JavaScript
 * Perfect for server-hosted JSON files with smart adaptive polling
 * Works on any static hosting (GitHub Pages, Netlify, Vercel, etc.)
 */

class GeometryHotReload {
    constructor(jsonPath = 'assets/sample_geometry.json', options = {}) {
        this.jsonPath = jsonPath;
        this.options = {
            minInterval: 8,       // ULTRA FAST polling (8ms = ~120 FPS!)
            maxInterval: 100,     // Quick recovery from idle
            backoffMultiplier: 1.1, // Minimal backoff for instant recovery
            resetAfterChange: true, // Reset to ultra-fast after change
            useVisibilityAPI: true, // Pause when tab hidden
            useConditionalRequests: true, // Use If-Modified-Since/ETag
            useRequestAnimationFrame: true, // Sync with display refresh rate
            useImmediateMode: true, // Skip setTimeout delays when possible
            fastFailover: true,   // Instant retry on network errors
            ...options
        };
        
        this.currentInterval = this.options.minInterval;
        this.lastContent = null;
        this.lastModified = null;
        this.lastETag = null;
        this.isPolling = false;
        this.reloadCallback = null;
        this.consecutiveUnchanged = 0;
        this.isVisible = true;
        this.timeoutId = null;
        
        this.setupVisibilityAPI();
        console.log('🔄 Optimized Hot Reload initialized for', jsonPath);
    }
    
    /**
     * Set callback function to call when geometry should be reloaded
     */
    setReloadCallback(callback) {
        this.reloadCallback = callback;
    }
    
    /**
     * Setup visibility API to pause when tab is hidden
     */
    setupVisibilityAPI() {
        if (!this.options.useVisibilityAPI || typeof document === 'undefined') {
            return;
        }
        
        document.addEventListener('visibilitychange', () => {
            this.isVisible = !document.hidden;
            if (this.isVisible && this.isPolling) {
                console.log('🔄 Tab visible - resuming hot reload');
                this.scheduleNextCheck(200); // Quick check when visible
            } else if (!this.isVisible) {
                console.log('🔄 Tab hidden - pausing hot reload');
            }
        });
    }
    
    /**
     * Start smart adaptive polling
     */
    start() {
        if (this.isPolling) {
            console.log('🔄 Hot reload already running');
            return;
        }
        
        this.isPolling = true;
        this.currentInterval = this.options.minInterval;
        this.consecutiveUnchanged = 0;
        
        console.log(`🔄 Starting smart hot reload (${this.options.minInterval}-${this.options.maxInterval}ms adaptive)`);
        
        // Initial check
        this.checkForChanges();
    }
    
    /**
     * Stop polling
     */
    stop() {
        if (!this.isPolling) return;
        
        this.isPolling = false;
        if (this.timeoutId) {
            clearTimeout(this.timeoutId);
            this.timeoutId = null;
        }
        console.log('🔄 Hot reload stopped');
    }
    
    /**
     * Check if JSON file has changed (with conditional requests)
     */
    async checkForChanges() {
        try {
            // Prepare headers for efficient conditional requests
            const headers = {
                'Cache-Control': 'no-cache, no-store, must-revalidate',
                'Pragma': 'no-cache',
                'Expires': '0'
            };
            
            // Add conditional request headers to minimize bandwidth
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
            
            // Handle 304 Not Modified (most efficient - no data transfer)
            if (response.status === 304) {
                this.handleNoChange();
                return;
            }
            
            if (!response.ok) {
                console.warn(`🔄 Failed to fetch ${this.jsonPath}: ${response.status}`);
                this.handleNoChange();
                return;
            }
            
            // Get caching headers for future requests
            const lastModified = response.headers.get('Last-Modified');
            const etag = response.headers.get('ETag');
            const content = await response.text();
            
            // Check if content actually changed
            if (this.hasContentChanged(content, lastModified, etag)) {
                console.log('🔄 Geometry file changed, reloading...');
                this.handleFileChanged(content, lastModified, etag);
            } else {
                this.handleNoChange();
            }
            
        } catch (error) {
            console.warn('🔄 Error checking for changes:', error.message);
            this.handleNoChange();
        }
    }
    
    /**
     * Handle file change - reload and reset to fast polling
     */
    async handleFileChanged(content, lastModified, etag) {
        // Update cached values
        this.lastContent = content;
        this.lastModified = lastModified;
        this.lastETag = etag;
        
        // Reset to fast polling after change
        if (this.options.resetAfterChange) {
            this.currentInterval = this.options.minInterval;
            this.consecutiveUnchanged = 0;
        }
        
        // Reload geometry
        if (this.reloadCallback) {
            try {
                await this.reloadCallback(JSON.parse(content));
                console.log('✅ Geometry reloaded successfully');
                
                // Show visual feedback
                this.showReloadFeedback();
            } catch (error) {
                console.error('❌ Failed to reload geometry:', error);
            }
        } else {
            console.warn('🔄 No reload callback set');
        }
        
        // Schedule next check
        this.scheduleNextCheck();
    }
    
    /**
     * Handle no change - apply exponential backoff
     */
    handleNoChange() {
        this.consecutiveUnchanged++;
        
        // Apply exponential backoff after several unchanged checks
        if (this.consecutiveUnchanged > 2) {
            const newInterval = Math.min(
                this.currentInterval * this.options.backoffMultiplier,
                this.options.maxInterval
            );
            
            if (newInterval !== this.currentInterval) {
                this.currentInterval = newInterval;
                console.log(`🔄 Backing off to ${Math.round(this.currentInterval)}ms polling`);
            }
        }
        
        // Schedule next check
        this.scheduleNextCheck();
    }
    
    /**
     * Schedule next check with adaptive timing
     */
    scheduleNextCheck(customDelay = null) {
        if (!this.isPolling) return;
        
        const delay = customDelay || this.currentInterval;
        
        // ULTRA REAL-TIME: Use immediate mode for fastest possible updates
        if (this.options.useImmediateMode && delay <= 16) {
            // For delays ≤16ms, use requestAnimationFrame for 60+ FPS sync
            if (this.options.useRequestAnimationFrame) {
                this.timeoutId = requestAnimationFrame(() => {
                    if (this.isVisible || !this.options.useVisibilityAPI) {
                        this.checkForChanges();
                    } else {
                        this.scheduleNextCheck();
                    }
                });
            } else {
                // Immediate execution for ultra-fast mode
                if (this.isVisible || !this.options.useVisibilityAPI) {
                    // Use setTimeout(0) for immediate but non-blocking execution
                    this.timeoutId = setTimeout(() => this.checkForChanges(), 0);
                } else {
                    this.scheduleNextCheck();
                }
            }
        } else {
            // Standard timeout for longer delays
            this.timeoutId = setTimeout(() => {
                if (this.isVisible || !this.options.useVisibilityAPI) {
                    this.checkForChanges();
                } else {
                    this.scheduleNextCheck();
                }
            }, delay);
        }
    }
    
    /**
     * Check if content has changed
     */
    hasContentChanged(content, lastModified, etag) {
        // First time loading
        if (this.lastContent === null) {
            return true;
        }
        
        // Compare by ETag (most reliable)
        if (etag && this.lastETag && etag !== this.lastETag) {
            return true;
        }
        
        // Compare by Last-Modified
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
     * Show visual feedback when geometry reloads
     */
    showReloadFeedback() {
        const indicator = document.getElementById('hot-reload-indicator');
        if (indicator) {
            indicator.style.background = '#4CAF50';
            indicator.innerHTML = '✅ Reloaded!';
            
            setTimeout(() => {
                indicator.style.background = '#2196F3';
                indicator.innerHTML = '🔄 Hot Reload Active';
            }, 2000);
        }
    }
    
    /**
     * Get current polling stats
     */
    getStats() {
        return {
            isPolling: this.isPolling,
            currentInterval: this.currentInterval,
            consecutiveUnchanged: this.consecutiveUnchanged,
            isVisible: this.isVisible
        };
    }
}

// Global instance
let geometryHotReload = null;

/**
 * Initialize hot reload with WASM geometry reload function
 */
function initGeometryHotReload(reloadFunction, options = {}) {
    if (geometryHotReload) {
        geometryHotReload.stop();
    }
    
    geometryHotReload = new GeometryHotReload('assets/sample_geometry.json', options);
    geometryHotReload.setReloadCallback(reloadFunction);
    geometryHotReload.start();
    
    // Add visual indicator
    addHotReloadIndicator();
    
    return geometryHotReload;
}

/**
 * Add visual indicator with controls
 */
function addHotReloadIndicator() {
    // Remove existing
    const existing = document.getElementById('hot-reload-indicator');
    if (existing) existing.remove();
    
    // Create indicator
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
        user-select: none;
    `;
    
    // Add click handler
    indicator.addEventListener('click', () => {
        if (!geometryHotReload) return;
        
        if (geometryHotReload.isPolling) {
            geometryHotReload.stop();
            indicator.innerHTML = '⏸️ Hot Reload Paused';
            indicator.style.background = '#FF9800';
        } else {
            geometryHotReload.start();
            indicator.innerHTML = '🔄 Hot Reload Active';
            indicator.style.background = '#2196F3';
        }
    });
    
    // Add stats on hover
    indicator.title = 'Click to toggle hot reload';
    
    document.body.appendChild(indicator);
}

// Export for modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { GeometryHotReload, initGeometryHotReload };
}

// Auto-detect and initialize if WASM is available
if (typeof window !== 'undefined') {
    window.GeometryHotReload = GeometryHotReload;
    window.initGeometryHotReload = initGeometryHotReload;
}
