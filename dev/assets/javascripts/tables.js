/**
 * Searchable, sortable, and paginated tables for documentation.
 *
 * Usage: Add class "searchable-table" to any table wrapper div.
 * Optional: Add data-filter-column="Status" to enable status dropdown filter.
 */

function initSearchableTables() {
  // Find all searchable table containers that haven't been initialized
  document.querySelectorAll('.searchable-table:not([data-initialized])').forEach(function(container) {
    var table = container.querySelector('table');
    if (!table) return;

    // Mark as initialized FIRST to prevent re-entry
    container.setAttribute('data-initialized', 'true');

    var thead = table.querySelector('thead');
    var tbody = table.querySelector('tbody');
    if (!tbody) return;

    var rows = Array.prototype.slice.call(tbody.querySelectorAll('tr'));
    if (rows.length === 0) return;

    var pageSize = parseInt(container.getAttribute('data-page-size')) || 10;

    var currentPage = 1;
    var filteredRows = rows.slice();
    var sortColumn = -1;
    var sortAscending = true;
    var currentSearchQuery = '';
    var currentStatusFilter = '';

    // Find the Status column index
    var statusColumnIndex = -1;
    var headerCells = thead ? thead.querySelectorAll('th') : [];
    headerCells.forEach(function(th, index) {
      var text = th.textContent.trim().toLowerCase();
      if (text === 'status') {
        statusColumnIndex = index;
      }
    });

    // Collect unique status values if status column exists
    var statusValues = [];
    if (statusColumnIndex >= 0) {
      var statusSet = {};
      rows.forEach(function(row) {
        var cell = row.cells[statusColumnIndex];
        if (cell) {
          var status = cell.textContent.trim();
          if (status && !statusSet[status]) {
            statusSet[status] = true;
            statusValues.push(status);
          }
        }
      });
      statusValues.sort();
    }

    // Create controls wrapper
    var controlsWrapper = document.createElement('div');
    controlsWrapper.className = 'table-controls';

    // Create search input
    var searchWrapper = document.createElement('div');
    searchWrapper.className = 'table-search-wrapper';
    searchWrapper.innerHTML = '<input type="text" class="table-search" placeholder="Filter..." aria-label="Filter table">';
    controlsWrapper.appendChild(searchWrapper);

    // Create status filter dropdown if status column exists
    var statusSelect = null;
    if (statusValues.length > 0) {
      var statusWrapper = document.createElement('div');
      statusWrapper.className = 'table-status-wrapper';
      var selectHtml = '<select class="table-status-filter" aria-label="Filter by status">';
      selectHtml += '<option value="">All Status</option>';
      statusValues.forEach(function(status) {
        selectHtml += '<option value="' + status + '">' + status + '</option>';
      });
      selectHtml += '</select>';
      statusWrapper.innerHTML = selectHtml;
      controlsWrapper.appendChild(statusWrapper);
      statusSelect = statusWrapper.querySelector('.table-status-filter');
    }

    container.prepend(controlsWrapper);

    // Create pagination controls - append to container
    var paginationWrapper = document.createElement('div');
    paginationWrapper.className = 'table-pagination';
    container.appendChild(paginationWrapper);

    var searchInput = searchWrapper.querySelector('.table-search');

    // Make headers sortable
    if (thead) {
      headerCells.forEach(function(th, index) {
        th.classList.add('sortable');
        th.setAttribute('data-col', index);
        th.innerHTML = '<span class="th-content">' + th.innerHTML + '</span><span class="sort-icon"></span>';
        th.addEventListener('click', function() {
          var col = parseInt(this.getAttribute('data-col'));
          if (sortColumn === col) {
            sortAscending = !sortAscending;
          } else {
            sortColumn = col;
            sortAscending = true;
          }
          // Update header classes
          headerCells.forEach(function(h) {
            h.classList.remove('sort-asc', 'sort-desc');
          });
          this.classList.add(sortAscending ? 'sort-asc' : 'sort-desc');
          sortRows();
        });
      });
    }

    function sortRows() {
      if (sortColumn < 0) return;

      filteredRows.sort(function(a, b) {
        var aCell = a.cells[sortColumn];
        var bCell = b.cells[sortColumn];
        if (!aCell || !bCell) return 0;

        var aText = aCell.textContent.trim().toLowerCase();
        var bText = bCell.textContent.trim().toLowerCase();

        // Try numeric comparison first
        var aNum = parseFloat(aText);
        var bNum = parseFloat(bText);
        if (!isNaN(aNum) && !isNaN(bNum)) {
          return sortAscending ? aNum - bNum : bNum - aNum;
        }

        // String comparison
        if (aText < bText) return sortAscending ? -1 : 1;
        if (aText > bText) return sortAscending ? 1 : -1;
        return 0;
      });

      // Re-append rows in sorted order
      filteredRows.forEach(function(row) {
        tbody.appendChild(row);
      });

      currentPage = 1;
      updateDisplay();
    }

    function updateDisplay() {
      // Hide all rows first
      rows.forEach(function(row) { row.style.display = 'none'; });

      // Calculate pagination
      var totalPages = Math.ceil(filteredRows.length / pageSize);
      var start = (currentPage - 1) * pageSize;
      var end = start + pageSize;

      // Show only current page rows
      filteredRows.slice(start, end).forEach(function(row) { row.style.display = ''; });

      // Update pagination controls
      renderPagination(totalPages);
    }

    function renderPagination(totalPages) {
      if (totalPages <= 1) {
        paginationWrapper.innerHTML = '<div class="pagination-controls"><span class="pagination-info">' + filteredRows.length + ' items</span></div>';
        return;
      }

      var html = '<div class="pagination-controls">';
      html += '<span class="pagination-info">' + filteredRows.length + ' items</span>';

      // Previous button
      html += '<button class="pagination-btn" data-page="prev"' + (currentPage === 1 ? ' disabled' : '') + '>&#8249;</button>';

      // Page numbers
      for (var i = 1; i <= totalPages; i++) {
        if (i === 1 || i === totalPages || (i >= currentPage - 1 && i <= currentPage + 1)) {
          html += '<button class="pagination-btn' + (i === currentPage ? ' active' : '') + '" data-page="' + i + '">' + i + '</button>';
        } else if (i === currentPage - 2 || i === currentPage + 2) {
          html += '<span class="pagination-ellipsis">...</span>';
        }
      }

      // Next button
      html += '<button class="pagination-btn" data-page="next"' + (currentPage === totalPages ? ' disabled' : '') + '>&#8250;</button>';
      html += '</div>';

      paginationWrapper.innerHTML = html;

      // Add click handlers
      paginationWrapper.querySelectorAll('.pagination-btn').forEach(function(btn) {
        btn.addEventListener('click', function() {
          var page = this.getAttribute('data-page');
          if (page === 'prev' && currentPage > 1) {
            currentPage--;
          } else if (page === 'next' && currentPage < totalPages) {
            currentPage++;
          } else if (page !== 'prev' && page !== 'next') {
            currentPage = parseInt(page);
          }
          updateDisplay();
        });
      });
    }

    function applyFilters() {
      var query = currentSearchQuery.toLowerCase().trim();
      var status = currentStatusFilter;

      filteredRows = rows.filter(function(row) {
        // Check text search
        var matchesSearch = !query || row.textContent.toLowerCase().indexOf(query) !== -1;

        // Check status filter
        var matchesStatus = true;
        if (status && statusColumnIndex >= 0) {
          var cell = row.cells[statusColumnIndex];
          matchesStatus = cell && cell.textContent.trim() === status;
        }

        return matchesSearch && matchesStatus;
      });

      // Re-apply sort if active
      if (sortColumn >= 0) {
        sortRows();
      } else {
        currentPage = 1;
        updateDisplay();
      }
    }

    // Set up search with debounce
    var debounceTimer;
    searchInput.addEventListener('input', function() {
      var value = this.value;
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(function() {
        currentSearchQuery = value;
        applyFilters();
      }, 150);
    });

    // Set up status filter
    if (statusSelect) {
      statusSelect.addEventListener('change', function() {
        currentStatusFilter = this.value;
        applyFilters();
      });
    }

    // Initial display
    updateDisplay();
  });
}

// Initialize immediately if DOM is ready, otherwise wait
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initSearchableTables);
} else {
  // DOM already loaded, run now
  initSearchableTables();
}

// For MkDocs Material instant navigation
// Subscribe to the document$ observable when available
document.addEventListener('DOMContentLoaded', function() {
  // Check for Material's instant loading
  if (typeof document$ !== 'undefined') {
    document$.subscribe(function() {
      initSearchableTables();
    });
  }

  // Fallback: watch for URL changes
  var lastPath = location.pathname;
  setInterval(function() {
    if (location.pathname !== lastPath) {
      lastPath = location.pathname;
      setTimeout(initSearchableTables, 50);
    }
  }, 200);
});
