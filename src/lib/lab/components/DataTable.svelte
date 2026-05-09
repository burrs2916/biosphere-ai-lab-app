<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Skeleton from '$lib/lab/components/Skeleton.svelte';
  import { t } from '$lib/i18n';

  export let columns: string[] = [];
  export let columnTypes: string[] = [];
  export let rows: any[][] = [];
  export let totalRows: number = 0;
  export let loading: boolean = false;
  export let pageSize: number = 50;
  export let currentPage: number = 0;
  export let columnProfiles: any[] = [];

  let sortColumn: string | null = null;
  let sortDirection: 'asc' | 'desc' = 'asc';
  let columnFilters: Record<string, string> = {};
  let globalSearch = '';
  let globalSearchDebounce: ReturnType<typeof setTimeout> | null = null;
  let filteredRows: any[][] = [];
  let visibleStart = 0;
  let visibleEnd = 50;
  const ROW_HEIGHT = 34;
  const VIRTUAL_BUFFER = 10;
  let scrollContainer: HTMLDivElement | null = null;
  let totalHeight = 0;
  let showColumnMenu: string | null = null;

  export let onPageChange: (page: number, pageSize: number) => void = () => {};
  export let onSortChange: (column: string, direction: 'asc' | 'desc') => void = () => {};
  export let onSearch: (query: string) => void = () => {};

  const totalPages = Math.max(1, Math.ceil(totalRows / pageSize));

  $: {
    let result = [...rows];
    if (globalSearch) {
      const q = globalSearch.toLowerCase();
      result = result.filter(row => row.some(cell => cell != null && String(cell).toLowerCase().includes(q)));
    }
    for (const [col, filter] of Object.entries(columnFilters)) {
      if (!filter) continue;
      const colIdx = columns.indexOf(col);
      if (colIdx < 0) continue;
      const f = filter.toLowerCase();
      if (f === '__null__') {
        result = result.filter(row => row[colIdx] == null);
      } else {
        result = result.filter(row => row[colIdx] != null && String(row[colIdx]).toLowerCase().includes(f));
      }
    }
    if (sortColumn) {
      const colIdx = columns.indexOf(sortColumn);
      if (colIdx >= 0) {
        result = [...result].sort((a, b) => {
          const va = a[colIdx];
          const vb = b[colIdx];
          if (va == null && vb == null) return 0;
          if (va == null) return 1;
          if (vb == null) return -1;
          const cmp = typeof va === 'number' && typeof vb === 'number' ? va - vb : String(va).localeCompare(String(vb));
          return sortDirection === 'asc' ? cmp : -cmp;
        });
      }
    }
    filteredRows = result;
    totalHeight = result.length * ROW_HEIGHT;
  }

  $: filteredTotal = filteredRows.length;

  function handleSort(col: string) {
    if (sortColumn === col) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumn = col;
      sortDirection = 'asc';
    }
    onSortChange(col, sortDirection);
  }

  function applyFilter(col: string) {}

  function clearFilter(col: string) {
    delete columnFilters[col];
    columnFilters = { ...columnFilters };
    showColumnMenu = null;
  }

  function handleGlobalSearch() {
    if (globalSearchDebounce) clearTimeout(globalSearchDebounce);
    globalSearchDebounce = setTimeout(() => {
      onSearch(globalSearch);
    }, 300);
  }

  function goToPage(page: number) {
    if (page >= 0 && page < totalPages) {
      onPageChange(page, pageSize);
    }
  }

  function handlePageSizeChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const newSize = parseInt(target.value);
    onPageChange(0, newSize);
  }

  function formatCell(val: any): string {
    if (val === null || val === undefined) return '—';
    if (typeof val === 'object') return JSON.stringify(val);
    return String(val);
  }

  function isNull(val: any): boolean {
    return val === null || val === undefined;
  }

  function sortIcon(col: string): string {
    if (sortColumn !== col) return '↕';
    return sortDirection === 'asc' ? '↑' : '↓';
  }

  function hasFilter(col: string): boolean {
    return !!columnFilters[col];
  }

  function handleScroll() {
    if (!scrollContainer) return;
    const scrollTop = scrollContainer.scrollTop;
    const containerHeight = scrollContainer.clientHeight;
    visibleStart = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - VIRTUAL_BUFFER);
    visibleEnd = Math.min(filteredRows.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + VIRTUAL_BUFFER);
  }

  function toggleColumnMenu(col: string) {
    showColumnMenu = showColumnMenu === col ? null : col;
  }

  function getColumnTypeIcon(type: string): string {
    switch (type) {
      case 'integer': case 'Integer': return '🔢';
      case 'float': case 'Float': return '📊';
      case 'string': case 'String': return '📝';
      case 'boolean': case 'Boolean': return '✅';
      case 'categorical': case 'Categorical': return '🏷️';
      case 'datetime': case 'DateTime': return '📅';
      default: return '❓';
    }
  }

  function getUniqueValues(colIdx: number): { value: string; count: number }[] {
    const counts: Record<string, number> = {};
    for (const row of filteredRows) {
      const v = row[colIdx];
      const key = v == null ? '(null)' : String(v);
      counts[key] = (counts[key] || 0) + 1;
    }
    return Object.entries(counts)
      .map(([value, count]) => ({ value, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 20);
  }

  function getNumericStats(colIdx: number) {
    const nums = filteredRows.map(r => r[colIdx]).filter(v => v != null && typeof v === 'number') as number[];
    if (nums.length === 0) return null;
    const min = Math.min(...nums);
    const max = Math.max(...nums);
    const mean = nums.reduce((a, b) => a + b, 0) / nums.length;
    const sorted = [...nums].sort((a, b) => a - b);
    const median = sorted.length % 2 === 0
      ? (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2
      : sorted[Math.floor(sorted.length / 2)];
    const nullCount = filteredRows.filter(r => r[colIdx] == null).length;
    return { min, max, mean, median, nullCount, count: nums.length };
  }

  function filterByValue(col: string, value: string) {
    if (value === '(null)') {
      columnFilters = { ...columnFilters, [col]: '__null__' };
    } else {
      columnFilters = { ...columnFilters, [col]: value };
    }
    showColumnMenu = null;
  }

  function clearAllFilters() {
    columnFilters = {};
    globalSearch = '';
    onSearch('');
  }

  function paginationRange(): (number | string)[] {
    const pages: (number | string)[] = [];
    const total = Math.max(1, Math.ceil(filteredTotal / pageSize));
    if (total <= 7) {
      for (let i = 0; i < total; i++) pages.push(i);
    } else {
      pages.push(0);
      if (currentPage > 2) pages.push('...');
      for (let i = Math.max(1, currentPage - 1); i <= Math.min(total - 2, currentPage + 1); i++) {
        pages.push(i);
      }
      if (currentPage < total - 3) pages.push('...');
      pages.push(total - 1);
    }
    return pages;
  }

  function handleClickOutside(e: MouseEvent) {
    if (showColumnMenu) {
      const target = e.target as HTMLElement;
      if (!target.closest('.col-menu-trigger') && !target.closest('.col-menu-popup')) {
        showColumnMenu = null;
      }
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
    if (globalSearchDebounce) clearTimeout(globalSearchDebounce);
  });
</script>

<div class="datatable">
  <div class="datatable-toolbar">
    <div class="datatable-info">
      {$t('dataTable.totalRows', { count: filteredTotal.toLocaleString() })}
      {#if globalSearch || Object.values(columnFilters).some(v => v)}
        <span class="filter-badge">{$t('dataTable.filteredOriginal', { count: totalRows.toLocaleString() })}</span>
        <button class="clear-all-filters" on:click={clearAllFilters}>{$t('dataTable.clearAllFilters')}</button>
      {/if}
    </div>
    <div class="datatable-controls">
      {#if rows.length > 0}
        <div class="global-search">
          <span class="gs-icon">🔍</span>
          <input
            type="text"
            placeholder={$t('dataTable.searchPlaceholder')}
            bind:value={globalSearch}
            on:input={handleGlobalSearch}
            class="gs-input"
          />
          {#if globalSearch}
            <button class="gs-clear" on:click={() => { globalSearch = ''; onSearch(''); }}>✕</button>
          {/if}
        </div>
      {/if}
      <label class="page-size-label">
        {$t('dataTable.perPage')}
        <select value={pageSize} on:change={handlePageSizeChange} class="page-size-select">
          <option value={25}>25</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
          <option value={200}>200</option>
        </select>
        {$t('dataTable.rows')}
      </label>
    </div>
  </div>

  <div class="datatable-wrapper" bind:this={scrollContainer} on:scroll={handleScroll}>
    {#if filteredRows.length > 500}
      <div class="virtual-scroll-container" style="height: {totalHeight}px; position: relative;">
        <table class="datatable-table" style="position: sticky; top: 0;">
          <thead>
            <tr>
              <th class="row-num-col">#</th>
              {#each columns as col, i}
                {@const colIdx = i}
                <th>
                  <div class="th-content">
                    <button class="th-sort-btn" on:click={() => handleSort(col)}>
                      <span class="th-type-icon">{getColumnTypeIcon(columnTypes[i])}</span>
                      <span class="th-name">{col}</span>
                      <span class="th-sort-icon" class:active-sort={sortColumn === col}>{sortIcon(col)}</span>
                    </button>
                    <button
                      class="th-filter-btn col-menu-trigger"
                      class:active={hasFilter(col) || showColumnMenu === col}
                      on:click|stopPropagation={() => toggleColumnMenu(col)}
                      title={$t('dataTable.colStatsFilter')}
                    >
                      {hasFilter(col) ? '🔍' : '☰'}
                    </button>
                  </div>
                  {#if showColumnMenu === col}
                    {@const isNum = columnTypes[i] === 'integer' || columnTypes[i] === 'float' || columnTypes[i] === 'Integer' || columnTypes[i] === 'Float'}
                    {@const numStats = isNum ? getNumericStats(colIdx) : null}
                    {@const uniqueVals = getUniqueValues(colIdx)}
                    {@const maxCount = Math.max(...uniqueVals.map(v => v.count), 1)}
                    <div class="col-menu-popup" on:click|stopPropagation>
                      <div class="stats-section">
                        <div class="stats-title">{$t('dataTable.statistics')}</div>
                        <div class="stats-row"><span class="stats-label">{$t('dataTable.nonNull')}</span><span class="stats-value">{filteredRows.filter(r => r[colIdx] != null).length}</span></div>
                        <div class="stats-row"><span class="stats-label">{$t('dataTable.nullValues')}</span><span class="stats-value">{filteredRows.filter(r => r[colIdx] == null).length}</span></div>
                        <div class="stats-row"><span class="stats-label">{$t('dataTable.uniqueValues')}</span><span class="stats-value">{uniqueVals.length}</span></div>
                        {#if numStats}
                            <div class="stats-row"><span class="stats-label">{$t('dataTable.min')}</span><span class="stats-value">{numStats.min.toFixed(2)}</span></div>
                            <div class="stats-row"><span class="stats-label">{$t('dataTable.max')}</span><span class="stats-value">{numStats.max.toFixed(2)}</span></div>
                            <div class="stats-row"><span class="stats-label">{$t('dataTable.mean')}</span><span class="stats-value">{numStats.mean.toFixed(2)}</span></div>
                            <div class="stats-row"><span class="stats-label">{$t('dataTable.median')}</span><span class="stats-value">{numStats.median.toFixed(2)}</span></div>
                        {/if}
                      </div>

                      <div class="stats-section">
                        <div class="stats-title">{$t('dataTable.valueDistribution')}</div>
                        <div class="value-list">
                          {#each uniqueVals as uv}
                            <div class="value-item" on:click={() => filterByValue(col, uv.value)} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && filterByValue(col, uv.value)}>
                              <span class="value-label">{uv.value.length > 12 ? uv.value.substring(0, 11) + '…' : uv.value}</span>
                              <div class="value-bar-bg">
                                <div class="value-bar-fill" style="width: {(uv.count / maxCount) * 100}%"></div>
                              </div>
                              <span class="value-count">{uv.count}</span>
                            </div>
                          {/each}
                        </div>
                      </div>

                      {#if hasFilter(col)}
                        <button class="clear-filter-btn" on:click={() => clearFilter(col)}>✕ {$t('dataTable.clearColFilter')}</button>
                      {/if}
                    </div>
                  {/if}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#if loading}
              {#each Array(10) as _}
                <tr style="height: {ROW_HEIGHT}px">
                  <td class="row-num-col"><Skeleton width="30px" height="16px" /></td>
                  {#each columns as _}
                    <td><Skeleton width="80px" height="16px" /></td>
                  {/each}
                </tr>
              {/each}
            {:else if filteredRows.length === 0}
              <tr>
                <td colspan={columns.length + 1} class="empty-cell">
                  {globalSearch || Object.values(columnFilters).some(v => v) ? $t('dataTable.noMatch') : $t('dataTable.noData')}
                </td>
              </tr>
            {:else}
              {#each filteredRows.slice(visibleStart, visibleEnd) as row, ri}
                <tr style="position: absolute; top: {(visibleStart + ri) * ROW_HEIGHT}px; left: 0; width: 100%; height: {ROW_HEIGHT}px;">
                  <td class="row-num-col">{visibleStart + ri + 1}</td>
                  {#each row as cell}
                    <td class="data-cell" class:null-cell={isNull(cell)} class:highlight={globalSearch && cell != null && String(cell).toLowerCase().includes(globalSearch.toLowerCase())}>
                      <span class="cell-value" title={formatCell(cell)}>{formatCell(cell)}</span>
                    </td>
                  {/each}
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {:else}
      <table class="datatable-table">
        <thead>
          <tr>
            <th class="row-num-col">#</th>
            {#each columns as col, i}
              {@const colIdx = i}
              <th>
                <div class="th-content">
                  <button class="th-sort-btn" on:click={() => handleSort(col)}>
                    <span class="th-type-icon">{getColumnTypeIcon(columnTypes[i])}</span>
                    <span class="th-name">{col}</span>
                    <span class="th-sort-icon" class:active-sort={sortColumn === col}>{sortIcon(col)}</span>
                  </button>
                  <button
                    class="th-filter-btn col-menu-trigger"
                    class:active={hasFilter(col) || showColumnMenu === col}
                    on:click|stopPropagation={() => toggleColumnMenu(col)}
                    title={$t('dataTable.colStatsFilter')}
                  >
                    {hasFilter(col) ? '🔍' : '☰'}
                  </button>
                </div>
                {#if showColumnMenu === col}
                  {@const isNum2 = columnTypes[i] === 'integer' || columnTypes[i] === 'float' || columnTypes[i] === 'Integer' || columnTypes[i] === 'Float'}
                  {@const numStats2 = isNum2 ? getNumericStats(colIdx) : null}
                  {@const uniqueVals2 = getUniqueValues(colIdx)}
                  {@const maxCount2 = Math.max(...uniqueVals2.map(v => v.count), 1)}
                  <div class="col-menu-popup" on:click|stopPropagation>
                    <div class="stats-section">
                      <div class="stats-title">{$t('dataTable.statistics')}</div>
                      <div class="stats-row"><span class="stats-label">{$t('dataTable.nonNull')}</span><span class="stats-value">{filteredRows.filter(r => r[colIdx] != null).length}</span></div>
                      <div class="stats-row"><span class="stats-label">{$t('dataTable.nullValues')}</span><span class="stats-value">{filteredRows.filter(r => r[colIdx] == null).length}</span></div>
                      <div class="stats-row"><span class="stats-label">{$t('dataTable.uniqueValues')}</span><span class="stats-value">{uniqueVals2.length}</span></div>
                      {#if numStats2}
                          <div class="stats-row"><span class="stats-label">{$t('dataTable.min')}</span><span class="stats-value">{numStats2.min.toFixed(2)}</span></div>
                          <div class="stats-row"><span class="stats-label">{$t('dataTable.max')}</span><span class="stats-value">{numStats2.max.toFixed(2)}</span></div>
                          <div class="stats-row"><span class="stats-label">{$t('dataTable.mean')}</span><span class="stats-value">{numStats2.mean.toFixed(2)}</span></div>
                          <div class="stats-row"><span class="stats-label">{$t('dataTable.median')}</span><span class="stats-value">{numStats2.median.toFixed(2)}</span></div>
                      {/if}
                    </div>

                    <div class="stats-section">
                      <div class="stats-title">{$t('dataTable.valueDistribution')}</div>
                      <div class="value-list">
                        {#each uniqueVals2 as uv}
                          <div class="value-item" on:click={() => filterByValue(col, uv.value)} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && filterByValue(col, uv.value)}>
                            <span class="value-label">{uv.value.length > 12 ? uv.value.substring(0, 11) + '…' : uv.value}</span>
                            <div class="value-bar-bg">
                              <div class="value-bar-fill" style="width: {(uv.count / maxCount2) * 100}%"></div>
                            </div>
                            <span class="value-count">{uv.count}</span>
                          </div>
                        {/each}
                      </div>
                    </div>

                    {#if hasFilter(col)}
                      <button class="clear-filter-btn" on:click={() => clearFilter(col)}>✕ {$t('dataTable.clearColFilter')}</button>
                    {/if}
                  </div>
                {/if}
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#if loading}
            {#each Array(Math.min(pageSize, 10)) as _}
              <tr>
                <td class="row-num-col"><Skeleton width="30px" height="16px" /></td>
                {#each columns as _}
                  <td><Skeleton width="80px" height="16px" /></td>
                {/each}
              </tr>
            {/each}
          {:else if filteredRows.length === 0}
            <tr>
              <td colspan={columns.length + 1} class="empty-cell">
                {globalSearch || Object.values(columnFilters).some(v => v) ? $t('dataTable.noMatch') : $t('dataTable.noData')}
              </td>
            </tr>
          {:else}
            {#each filteredRows as row, ri}
              <tr>
                <td class="row-num-col">{ri + 1}</td>
                {#each row as cell}
                  <td class="data-cell" class:null-cell={isNull(cell)} class:highlight={globalSearch && cell != null && String(cell).toLowerCase().includes(globalSearch.toLowerCase())}>
                    <span class="cell-value" title={formatCell(cell)}>{formatCell(cell)}</span>
                  </td>
                {/each}
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    {/if}
  </div>

  <div class="datatable-footer">
    <div class="pagination">
      <button class="page-btn" disabled={currentPage === 0} on:click={() => goToPage(0)}>««</button>
      <button class="page-btn" disabled={currentPage === 0} on:click={() => goToPage(currentPage - 1)}>«</button>
      {#each paginationRange() as page}
        {#if page === '...'}
          <span class="page-ellipsis">...</span>
        {:else}
          <button class="page-btn" class:active={page === currentPage} on:click={() => goToPage(page as number)}>
            {(page as number) + 1}
          </button>
        {/if}
      {/each}
      <button class="page-btn" disabled={currentPage >= totalPages - 1} on:click={() => goToPage(currentPage + 1)}>»</button>
      <button class="page-btn" disabled={currentPage >= totalPages - 1} on:click={() => goToPage(totalPages - 1)}>»»</button>
    </div>
    <div class="page-jump">
      {$t('dataTable.goTo')}
      <input type="number" class="jump-input" min={1} max={totalPages} placeholder={String(currentPage + 1)} on:keydown={(e) => { if (e.key === 'Enter') { const val = parseInt((e.target as HTMLInputElement).value); if (val >= 1 && val <= totalPages) goToPage(val - 1); } }} />
      {$t('dataTable.page')}
    </div>
  </div>
</div>

<style>
  .datatable {
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 8px;
    overflow: hidden;
    background: rgba(15, 23, 42, 0.3);
  }

  .datatable-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.08);
    background: rgba(255, 255, 255, 0.02);
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .datatable-info {
    font-size: 0.78rem;
    color: #94a3b8;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .datatable-info strong { color: #e2e8f0; }
  .filter-badge { color: #93c5fd; font-size: 0.72rem; }

  .clear-all-filters {
    background: none;
    border: none;
    color: #f59e0b;
    font-size: 0.72rem;
    cursor: pointer;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    text-decoration: underline;
  }

  .clear-all-filters:hover { color: #fbbf24; }

  .datatable-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .global-search {
    position: relative;
    display: flex;
    align-items: center;
  }

  .gs-icon {
    position: absolute;
    left: 0.5rem;
    font-size: 0.75rem;
    pointer-events: none;
  }

  .gs-input {
    padding: 0.3rem 1.5rem 0.3rem 1.8rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 5px;
    color: #e2e8f0;
    font-size: 0.75rem;
    outline: none;
    width: 200px;
    transition: border-color 0.15s, width 0.2s;
  }

  .gs-input:focus { border-color: #3b82f6; width: 260px; }
  .gs-input::placeholder { color: #6b7280; }

  .gs-clear {
    position: absolute;
    right: 0.35rem;
    background: none;
    border: none;
    color: #6b7280;
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0.1rem;
  }

  .gs-clear:hover { color: #d1d5db; }

  .page-size-label {
    font-size: 0.75rem;
    color: #94a3b8;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .page-size-select {
    padding: 0.15rem 0.3rem;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 4px;
    color: #e2e8f0;
    font-size: 0.75rem;
    outline: none;
  }

  .datatable-wrapper {
    overflow-x: auto;
    max-height: 500px;
    overflow-y: auto;
  }

  .virtual-scroll-container { overflow: hidden; position: relative; }

  .datatable-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.78rem;
    table-layout: auto;
    min-width: 100%;
  }

  .datatable-table thead {
    position: sticky;
    top: 0;
    z-index: 5;
  }

  .datatable-table th {
    background: rgba(30, 41, 59, 0.95);
    backdrop-filter: blur(8px);
    padding: 0;
    border-bottom: 2px solid rgba(148, 163, 184, 0.15);
    white-space: nowrap;
    position: relative;
  }

  .th-content {
    display: flex;
    align-items: center;
    padding: 0.4rem 0.5rem;
    gap: 0.25rem;
  }

  .th-sort-btn {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    background: none;
    border: none;
    color: #9ca3af;
    font-size: 0.72rem;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
    text-align: left;
    flex: 1;
    min-width: 0;
  }

  .th-sort-btn:hover { color: #e2e8f0; }
  .th-type-icon { font-size: 0.65rem; flex-shrink: 0; }

  .th-name {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 150px;
  }

  .th-sort-icon {
    font-size: 0.65rem;
    color: #475569;
    flex-shrink: 0;
    transition: color 0.15s;
  }

  .th-sort-icon.active-sort { color: #3b82f6; }

  .th-filter-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.65rem;
    padding: 0.1rem 0.2rem;
    border-radius: 3px;
    opacity: 0.5;
    flex-shrink: 0;
    transition: opacity 0.15s, background 0.15s;
  }

  .th-filter-btn:hover,
  .th-filter-btn.active {
    opacity: 1;
    background: rgba(59, 130, 246, 0.1);
  }

  .col-menu-popup {
    position: absolute;
    top: 100%;
    right: 0;
    width: 280px;
    max-height: 400px;
    overflow-y: auto;
    background: rgba(30, 41, 59, 0.98);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 6px;
    padding: 0.5rem;
    z-index: 20;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .stats-section { margin-bottom: 0.5rem; }
  .stats-title { font-size: 0.7rem; color: #94a3b8; font-weight: 600; margin-bottom: 0.3rem; text-transform: uppercase; letter-spacing: 0.04em; }
  .stats-row { display: flex; justify-content: space-between; font-size: 0.72rem; color: #d1d5db; padding: 0.15rem 0; }
  .stats-label { color: #9ca3af; }
  .stats-value { color: #e2e8f0; font-variant-numeric: tabular-nums; }

  .value-list { display: flex; flex-direction: column; gap: 0.1rem; }
  .value-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0.3rem;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.7rem;
    color: #d1d5db;
    transition: background 0.1s;
  }

  .value-item:hover { background: rgba(59, 130, 246, 0.1); }
  .value-label { min-width: 60px; max-width: 80px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .value-bar-bg {
    flex: 1;
    height: 4px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 2px;
    overflow: hidden;
  }

  .value-bar-fill {
    height: 100%;
    background: #3b82f6;
    border-radius: 2px;
    transition: width 0.2s;
  }

  .value-count { color: #64748b; font-size: 0.65rem; min-width: 30px; text-align: right; }

  .clear-filter-btn {
    width: 100%;
    padding: 0.3rem;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 4px;
    color: #fca5a5;
    font-size: 0.72rem;
    cursor: pointer;
    margin-top: 0.3rem;
  }

  .clear-filter-btn:hover { background: rgba(239, 68, 68, 0.15); }

  .row-num-col {
    width: 50px;
    text-align: right;
    padding: 0.3rem 0.5rem;
    color: #64748b;
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
    border-right: 1px solid rgba(148, 163, 184, 0.06);
  }

  .datatable-table td {
    padding: 0.3rem 0.5rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.04);
    color: #d1d5db;
    max-width: 250px;
  }

  .datatable-table tbody tr:hover { background: rgba(59, 130, 246, 0.03); }
  .data-cell { overflow: hidden; }

  .cell-value {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 250px;
  }

  .null-cell .cell-value { color: #64748b; font-style: italic; }
  .highlight .cell-value { background: rgba(245, 158, 11, 0.2); border-radius: 2px; padding: 0 2px; }
  .empty-cell { text-align: center; color: #64748b; padding: 2rem !important; }

  .datatable-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border-top: 1px solid rgba(148, 163, 184, 0.08);
    background: rgba(255, 255, 255, 0.02);
  }

  .pagination { display: flex; align-items: center; gap: 0.15rem; }

  .page-btn {
    padding: 0.2rem 0.4rem;
    font-size: 0.7rem;
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.03);
    color: #94a3b8;
    cursor: pointer;
    min-width: 24px;
    text-align: center;
  }

  .page-btn:hover:not(:disabled) { background: rgba(255, 255, 255, 0.06); color: #e2e8f0; }
  .page-btn.active { background: rgba(59, 130, 246, 0.15); border-color: rgba(59, 130, 246, 0.3); color: #93c5fd; }
  .page-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .page-ellipsis { padding: 0 0.2rem; color: #64748b; font-size: 0.7rem; }

  .page-jump { display: flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; color: #94a3b8; }

  .jump-input {
    width: 40px;
    padding: 0.15rem 0.3rem;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 3px;
    color: #e2e8f0;
    font-size: 0.7rem;
    text-align: center;
    outline: none;
  }

  .jump-input:focus { border-color: rgba(59, 130, 246, 0.4); }
</style>
