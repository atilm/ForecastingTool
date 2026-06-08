* Add a new estimate type HistogramEstimate which contains a histogram loaded from a referenced
  file.
* EstimateRecord should have optional fields report_file and histogram_file. 
  * If a histogram_file is given, then a HistogramEstimate should be parsed.
* sample_duration.rs could sample from the HistogramEstimate and generate a triplet with all-equal entries,
  then rely on the fact that the ThreePointSampler will just return the value if optimistic = pessimistic