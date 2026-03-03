import json
import logging
import re
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

LOGIN_ENDPOINT = '/session/v1/login-request'
QUERY_REQUEST_ENDPOINT = '/queries/v1/query-request'
QUERY_RESULT_PATTERN = re.compile(r'/queries/[0-9a-f-]+/result')

DATA_CHUNK_PATTERN = re.compile(r'data_\d+_\d+_\d+_\d+')


class MappingTransformer:
    """Transform WireMock mappings to be more generic"""
    
    @staticmethod
    def transform_mappings_directory(mappings_dir: Path) -> int:
        """
        Transform all mapping files in a directory.
        
        Args:
            mappings_dir: Directory containing WireMock mapping JSON files
        
        Returns:
            Number of mappings transformed
        """
        if not mappings_dir.exists():
            logger.warning(f"Mappings directory does not exist: {mappings_dir}")
            return 0
        
        mapping_files = list(mappings_dir.glob("*.json"))
        
        # Process all mappings - check removal first, then transform
        stats = {'transformed': 0, 'removed': 0, 'errors': 0}
        
        for mapping_file in mapping_files:
            try:
                with open(mapping_file, 'r') as f:
                    data = json.load(f)
                
                # Check if mapping should be removed (ALTER SESSION query)
                if MappingTransformer._is_alter_session(data):
                    mapping_file.unlink()
                    stats['removed'] += 1
                else:
                    # Transform the mapping (only if not removed)
                    transformed = MappingTransformer._transform_mapping_content(data)
                    with open(mapping_file, 'w') as f:
                        json.dump(transformed, f, indent=2)
                    stats['transformed'] += 1
                    
            except (json.JSONDecodeError, OSError) as e:
                stats['errors'] += 1
                logger.error(f"Failed to process {mapping_file.name}: {e}")
            except Exception as e:
                stats['errors'] += 1
                logger.error(f"Unexpected error processing {mapping_file.name}: {e}", exc_info=True)
        
        logger.info(f"✓ Processed {stats['transformed'] + stats['removed'] + stats['errors']} files: "
                   f"{stats['transformed']} transformed, {stats['removed']} removed, {stats['errors']} errors")
        return stats['transformed']

    @staticmethod
    def _transform_mapping_content(mapping: dict[str, Any]) -> dict[str, Any]:
        mapping['persistent'] = False
        
        request = mapping.get('request', {})
        if not request:
            return mapping
        # Extract URL information once (avoid variable shadowing)
        url_path = request.get('urlPath', '')
        url_exact = request.get('url', '')
        url_pattern = request.get('urlPattern', '')
        current_url = url_path or url_exact or url_pattern
        url_field = 'urlPath' if url_path else ('url' if url_exact else ('urlPattern' if url_pattern else None))
        
        # Handle login request
        if LOGIN_ENDPOINT in current_url:
            MappingTransformer._remove_body_patterns(request)
            MappingTransformer._remove_query_parameters(request)
            mapping['request'] = request
            mapping['priority'] = 10
            return mapping
        
        # Handle query requests
        if QUERY_REQUEST_ENDPOINT in current_url:
            MappingTransformer._remove_body_patterns(request)
            MappingTransformer._remove_query_parameters(request)
            mapping['request'] = request
            mapping['priority'] = 5
            return mapping
        
        # Handle query result endpoints
        if QUERY_RESULT_PATTERN.search(current_url):
            # Transform /queries/{queryId}/result -> /queries/.*/result.*
            mapping['request']['urlPattern'] = '/queries/.*/result.*'
            if 'url' in mapping['request']:
                del mapping['request']['url']
            if 'urlPath' in mapping['request']:
                del mapping['request']['urlPath']
            mapping['priority'] = 5
            return mapping
        
        # Transform request URL patterns for all other endpoints
        if url_field and current_url:
            if MappingTransformer._is_data_chunk(current_url):
                # Data chunk: data_1234_5678_9012_3456 -> .*data_1234_5678_9012_3456.*
                pattern = MappingTransformer._extract_data_chunk_pattern(current_url)
                if pattern:
                    mapping['request']['urlPattern'] = pattern
                    if url_field != 'urlPattern' and url_field in mapping['request']:
                        del mapping['request'][url_field]
                # OPTIMIZATION 1: Remove ALL query parameters for data chunks (faster matching)
                MappingTransformer._remove_query_parameters(request)
                # OPTIMIZATION 2: Assign priority (data chunks are MOST common - check first)
                mapping['priority'] = 1
            else:
                # Other URLs: add wildcard for query parameters
                base_path = current_url.split('?')[0] if '?' in current_url else current_url
                mapping['request']['urlPattern'] = f"{base_path}.*"
                if url_field != 'urlPattern' and url_field in mapping['request']:
                    del mapping['request'][url_field]
                # Remove ALL query parameters for faster matching
                MappingTransformer._remove_query_parameters(request)
                # OPTIMIZATION 2: Assign priority (other requests are medium-low frequency)
                mapping['priority'] = 8
        
        return mapping
    
    @staticmethod
    def _remove_body_patterns(request: dict[str, Any]) -> None:
        """Remove bodyPatterns from request (in-place)."""
        if 'bodyPatterns' in request:
            del request['bodyPatterns']
    
    @staticmethod
    def _remove_query_parameters(request: dict[str, Any]) -> None:
        """Remove all queryParameters from request (in-place)."""
        if 'queryParameters' in request:
            del request['queryParameters']
    
    @staticmethod
    def _is_alter_session(mapping: dict[str, Any]) -> bool:
        """
        Detect ALTER SESSION queries using two heuristics:
        1. Text-based: "ALTER SESSION" appears in body/URL/response
        2. Pattern-based: Response has 1 row with no data chunks
        """
        # Heuristic 1: Check for "ALTER SESSION" text
        if 'request' in mapping:
            # Check body patterns
            body_patterns = mapping['request'].get('bodyPatterns', [])
            for pattern in body_patterns:
                if 'alter session' in str(pattern).lower():
                    return True
            
            # Check URL
            url = mapping['request'].get('url', '') + mapping['request'].get('urlPattern', '')
            if 'alter session' in url.lower():
                return True
        
        # Check response body for ALTER SESSION in sqlText
        response = mapping.get('response', {})
        response_body = response.get('body', '')
        if response_body and 'alter session' in response_body.lower():
            return True
        
        # Heuristic 2: Check response characteristics (1 row, no chunks)
        if not response_body:
            return False
        
        try:
            response_json = json.loads(response_body)
            data = response_json.get('data', {})
            
            # Handle case where data is explicitly None in JSON
            if data is None:
                return False
            
            # ALTER queries return 1 row, no chunks
            total = data.get('total', 0)
            returned = data.get('returned', 0)
            chunks = data.get('chunks', [])
            
            if total == 1 and returned == 1 and len(chunks) == 0:
                return True
        except (json.JSONDecodeError, KeyError, TypeError):
            # Malformed or unexpected response body — not a metadata-only response
            pass
        
        return False
    
    @staticmethod
    def _is_data_chunk(url: str) -> bool:
        """Check if URL is a data chunk download"""
        return bool(DATA_CHUNK_PATTERN.search(url))
    
    @staticmethod
    def _extract_data_chunk_pattern(url: str) -> str:
        """
        Extract data chunk pattern from URL.
        
        Example: /path/data_1234_5678_9012_3456.gz -> .*data_1234_5678_9012_3456.*
        """
        match = DATA_CHUNK_PATTERN.search(url)
        if match:
            return f".*{match.group(0)}.*"
        return url
