# EPICS Archiver Appliance API Client

A high-performance, Rust-based client for the EPICS Archiver Appliance with support for parallel requests, intelligent caching, and optimized data retrieval.

## Features

- **Concurrent Data Retrieval**: Processes multiple PV requests in parallel with smart rate limiting
- **Intelligent Caching**: Uses DashMap for thread-safe, high-performance data caching
- **Automatic Optimization**: Smart data binning based on time ranges
- **Real-time Updates**: Support for live data streaming with automatic cache updates
- **Thread-safe**: All operations are designed for concurrent access
- **Memory Efficient**: Optimized data structures and automatic cache cleanup
- **Error Resilient**: Comprehensive error handling and recovery mechanisms

## Architecture

### Core Components

1. **ArchiverClient**
   - Main interface for interacting with the EPICS Archiver
   - Handles request coordination and resource management
   - Manages concurrent operations and rate limiting

2. **Data Processor**
   - Processes raw PV data
   - Validates data against display ranges
   - Calculates statistics and normalizes data formats

3. **Cache System**
   - Uses DashMap for thread-safe concurrent access
   - Automatic expiration of stale data
   - Periodic cleanup to prevent memory leaks

### Optimization Features

#### 1. DashMap Cache
```rust
data_cache: Arc<DashMap<String, CacheEntry>>
```
- Why DashMap?
  - Thread-safe without global locks
  - Better performance than RwLock<HashMap> for concurrent access
  - Per-bucket locking reduces contention
  - Optimized for read-heavy workloads
  - Built-in support for concurrent iterations

#### 2. Parallel Request Processing
```rust
const MAX_CONCURRENT_REQUESTS: usize = 10;
```
- Chunks large PV lists into optimal batch sizes
- Uses semaphore for rate limiting
- Prevents server overload while maximizing throughput
- Automatic request retry with backoff

#### 3. Data Optimization Levels
```rust
pub enum OptimizationLevel {
    Raw,
    Optimized(i32),
    Auto,
}
```
- **Raw**: No optimization, full data resolution
- **Optimized**: Fixed number of points with binning
- **Auto**: Intelligent binning based on time range:
  - ≤ 1 day: Raw data
  - ≤ 7 days: 1-minute bins
  - ≤ 30 days: 15-minute bins
  - > 30 days: 1-hour bins

### Caching Strategy

1. **Cache Entry Structure**
```rust
struct CacheEntry {
    data: NormalizedPVData,
    expires_at: SystemTime,
}
```

2. **Cache Key Format**
```
{pv_name}:{start_time}:{end_time}:{optimization_level}
```

3. **Cache Management**
- TTL-based expiration (default 5 minutes)
- Automatic cleanup during live updates
- Manual cleanup method available
- Cache statistics tracking

### Live Updates

1. **Update Modes**
- Rolling window: Fixed-size moving time window
- Append: Continuous data accumulation

2. **Performance Features**
- Efficient broadcast channel for updates
- Automatic cache updates during live streaming
- Smart connection management
- Graceful shutdown handling

## Usage Examples

### Basic Data Retrieval
```rust
let client = ArchiverClient::new()?;
let data = client.fetch_historical_data(
    "SOME:PV:NAME",
    &TimeRangeMode::Fixed { 
        start: start_time, 
        end: end_time 
    },
    OptimizationLevel::Auto,
    Some(800), // chart width
    Some("America/Los_Angeles"),
).await?;
```

### Multiple PV Retrieval
```rust
let pvs = vec!["PV1", "PV2", "PV3"];
let data = client.fetch_multiple_pvs(
    &pvs,
    &time_range,
    OptimizationLevel::Auto,
    Some(chart_width),
    Some(timezone),
).await?;
```

### Live Updates
```rust
let rx = client.start_live_updates(
    pvs,
    Duration::from_secs(1),
    Some("UTC".to_string()),
).await?;

while let Ok(update) = rx.recv().await {
    // Process live data
}
```

## Performance Considerations

1. **Network Optimization**
- Request batching for multiple PVs
- Connection pooling
- Parallel request processing
- Rate limiting to prevent overload

2. **Memory Management**
- Efficient data structures
- Automatic cache cleanup
- Smart memory allocation
- Buffer reuse when possible

3. **CPU Optimization**
- Parallel processing where beneficial
- Efficient data binning algorithms
- Smart use of async/await
- Minimal data copying

## Error Handling

1. **Network Errors**
- Automatic retry with backoff
- Connection timeout handling
- Rate limit detection

2. **Data Validation**
- Range checking
- Timestamp validation
- Data format verification

3. **Resource Management**
- Graceful shutdown
- Resource cleanup
- Memory overflow prevention

## Best Practices

1. **Configuration**
- Adjust MAX_CONCURRENT_REQUESTS based on server capacity
- Set appropriate CACHE_TTL for your use case
- Configure optimization levels based on data requirements

2. **Usage Patterns**
- Use batch requests for multiple PVs
- Implement error handling for network issues
- Monitor cache statistics for optimization
- Clean up resources after use

3. **Performance Tips**
- Use appropriate optimization levels
- Batch related PV requests
- Implement proper error handling
- Monitor resource usage

## Implementation Details

### Cache Key Generation
```rust
let cache_key = format!("{}:{}:{}:{:?}", pv, start, end, optimization);
```

### Rate Limiting
```rust
let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
```

### Data Processing
```rust
pub fn process_data(&self, data: PVData) -> Result<NormalizedPVData, String>
```

## Contributing

1. **Adding Features**
- Follow existing error handling patterns
- Maintain thread safety
- Add appropriate tests
- Document performance implications

2. **Testing**
- Add unit tests for new features
- Include performance tests
- Test edge cases
- Verify thread safety